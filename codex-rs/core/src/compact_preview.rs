use std::sync::Arc;

use crate::Prompt;
use crate::client_common::ResponseEvent;
use crate::codex::Session;
use crate::codex::TurnContext;
use crate::codex::get_last_assistant_message_from_turn;
use crate::error::CodexErr;
use crate::error::Result as CodexResult;
use crate::util::backoff;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::user_input::UserInput;
use futures::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompactionCarryOverStats {
    pub(crate) selected_count: usize,
    pub(crate) total_count: usize,
    pub(crate) was_truncated: bool,
}

impl CompactionCarryOverStats {
    pub(crate) fn percent(&self) -> u64 {
        let denom = self.total_count.max(1) as f64;
        ((100.0 * (self.selected_count as f64) / denom).round() as u64).min(100)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CompactionPreview {
    Local {
        summary_text: String,
        carry_over: CompactionCarryOverStats,
    },
    Remote {
        replacement_history: Vec<ResponseItem>,
        carry_over: CompactionCarryOverStats,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompactionPreviewOutput {
    pub(crate) summary_text: String,
    pub(crate) carry_over: CompactionCarryOverStats,
    pub(crate) is_remote: bool,
}

pub(crate) async fn generate_local_preview(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    input: Vec<UserInput>,
) -> CodexResult<CompactionPreviewOutput> {
    let initial_input_for_turn: ResponseInputItem = ResponseInputItem::from(input);

    let mut history = sess.clone_history().await;
    history.record_items(
        &[initial_input_for_turn.into()],
        turn_context.truncation_policy,
    );

    let mut truncated_count = 0usize;

    let max_retries = turn_context.client.get_provider().stream_max_retries();
    let mut retries = 0;

    loop {
        let turn_input = history.get_history_for_prompt();
        let prompt = Prompt {
            input: turn_input.clone(),
            ..Default::default()
        };
        let attempt_result = drain_preview_to_completed(&mut history, &turn_context, &prompt).await;

        match attempt_result {
            Ok(()) => {
                break;
            }
            Err(CodexErr::Interrupted) => {
                return Err(CodexErr::Interrupted);
            }
            Err(e @ CodexErr::ContextWindowExceeded) => {
                if turn_input.len() > 1 {
                    history.remove_first_item();
                    truncated_count += 1;
                    retries = 0;
                    continue;
                }
                return Err(e);
            }
            Err(e) => {
                if retries < max_retries {
                    retries += 1;
                    let delay = backoff(retries);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(e);
            }
        }
    }

    let _ = truncated_count;

    let history_snapshot = history.get_history();
    let summary_suffix =
        get_last_assistant_message_from_turn(&history_snapshot).unwrap_or_default();
    let summary_text = format!("{SUMMARY_PREFIX}\n{summary_suffix}");
    let user_messages = crate::compact::collect_user_messages(&history_snapshot);
    let carry_over = compute_carry_over(&user_messages, COMPACT_USER_MESSAGE_MAX_TOKENS);

    Ok(CompactionPreviewOutput {
        summary_text,
        carry_over,
        is_remote: false,
    })
}

pub(crate) fn preview_from_remote_replacement_history(
    replacement_history: Vec<ResponseItem>,
) -> CompactionPreviewOutput {
    let summary_text = extract_remote_summary_text(&replacement_history);
    let user_messages = crate::compact::collect_user_messages(&replacement_history);
    let carry_over = compute_carry_over(&user_messages, COMPACT_USER_MESSAGE_MAX_TOKENS);

    CompactionPreviewOutput {
        summary_text,
        carry_over,
        is_remote: true,
    }
}

pub(crate) fn extract_remote_summary_text(replacement_history: &[ResponseItem]) -> String {
    let mut found_summary = None;
    for item in replacement_history {
        if let ResponseItem::Message { role, content, .. } = item
            && role == "user"
            && found_summary.is_none()
        {
            found_summary = content_items_to_text(content).map(|s| s.trim().to_string());
        }

        if matches!(item, ResponseItem::Compaction { .. }) {
            break;
        }
    }

    match found_summary {
        Some(summary) if !summary.is_empty() => summary,
        _ => "(compaction preview summary unavailable; apply still works)".to_string(),
    }
}

fn compute_carry_over(user_messages: &[String], max_tokens: usize) -> CompactionCarryOverStats {
    if max_tokens == 0 {
        return CompactionCarryOverStats {
            selected_count: 0,
            total_count: user_messages.len(),
            was_truncated: false,
        };
    }

    let mut remaining = max_tokens;
    let mut selected_count = 0usize;
    let mut was_truncated = false;

    for message in user_messages.iter().rev() {
        if remaining == 0 {
            break;
        }

        let tokens = crate::truncate::approx_token_count(message);
        if tokens <= remaining {
            selected_count += 1;
            remaining = remaining.saturating_sub(tokens);
        } else {
            selected_count += 1;
            was_truncated = true;
            break;
        }
    }

    CompactionCarryOverStats {
        selected_count,
        total_count: user_messages.len(),
        was_truncated,
    }
}

fn content_items_to_text(content: &[ContentItem]) -> Option<String> {
    crate::compact::content_items_to_text(content)
}

async fn drain_preview_to_completed(
    history: &mut crate::context_manager::ContextManager,
    turn_context: &TurnContext,
    prompt: &Prompt,
) -> CodexResult<()> {
    let mut stream = turn_context.client.clone().stream(prompt).await?;
    loop {
        let maybe_event = stream.next().await;
        let Some(event) = maybe_event else {
            return Err(CodexErr::Stream(
                "stream closed before response.completed".into(),
                None,
            ));
        };
        match event {
            Ok(ResponseEvent::OutputItemDone(item)) => {
                history.record_items(std::slice::from_ref(&item), turn_context.truncation_policy);
            }
            Ok(ResponseEvent::RateLimits(snapshot)) => {
                // Preview does not update session state.
                let _ = snapshot;
            }
            Ok(ResponseEvent::Completed { token_usage, .. }) => {
                let _ = token_usage;
                return Ok(());
            }
            Ok(_) => continue,
            Err(e) => return Err(e),
        }
    }
}

pub(crate) const SUMMARY_PREFIX: &str = crate::compact::SUMMARY_PREFIX;
pub(crate) const COMPACT_USER_MESSAGE_MAX_TOKENS: usize = 20_000;
