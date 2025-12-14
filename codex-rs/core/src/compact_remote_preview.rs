use std::sync::Arc;

use crate::Prompt;
use crate::codex::Session;
use crate::codex::TurnContext;
use crate::error::Result as CodexResult;
use codex_protocol::models::ResponseItem;

use crate::compact_preview::CompactionPreview;
use crate::compact_preview::preview_from_remote_replacement_history;

pub(crate) async fn run_remote_compact_preview(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
) -> CodexResult<CompactionPreview> {
    let mut history = sess.clone_history().await;
    let prompt = Prompt {
        input: history.get_history_for_prompt(),
        tools: vec![],
        parallel_tool_calls: false,
        base_instructions_override: turn_context.base_instructions.clone(),
        output_schema: None,
    };

    let mut replacement_history = turn_context
        .client
        .compact_conversation_history(&prompt)
        .await?;

    // Required to keep `/undo` available after compaction.
    let ghost_snapshots: Vec<ResponseItem> = history
        .get_history()
        .iter()
        .filter(|item| matches!(item, ResponseItem::GhostSnapshot { .. }))
        .cloned()
        .collect();

    if !ghost_snapshots.is_empty() {
        replacement_history.extend(ghost_snapshots);
    }

    let output = preview_from_remote_replacement_history(replacement_history.clone());
    Ok(CompactionPreview::Remote {
        replacement_history,
        carry_over: output.carry_over,
    })
}
