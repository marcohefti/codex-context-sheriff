use std::sync::Arc;

use crate::codex::Session;
use crate::codex::TurnContext;
use crate::error::Result as CodexResult;
use crate::protocol::CompactedItem;
use crate::protocol::ContextCompactedEvent;
use crate::protocol::EventMsg;
use crate::protocol::RolloutItem;
use crate::protocol::WarningEvent;
use codex_protocol::models::ResponseItem;

use crate::compact_preview::CompactionPreview;

pub(crate) async fn apply_compaction_preview(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    preview: CompactionPreview,
) -> CodexResult<()> {
    match preview {
        CompactionPreview::Local {
            summary_text,
            carry_over: _,
        } => apply_local(sess, turn_context, summary_text).await,
        CompactionPreview::Remote {
            replacement_history,
            carry_over: _,
        } => apply_remote(sess, turn_context, replacement_history).await,
    }
}

async fn apply_local(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    summary_text: String,
) -> CodexResult<()> {
    let history_snapshot = sess.clone_history().await.get_history();
    let user_messages = crate::compact::collect_user_messages(&history_snapshot);
    let initial_context = sess.build_initial_context(turn_context.as_ref());
    let mut new_history =
        crate::compact::build_compacted_history(initial_context, &user_messages, &summary_text);
    let ghost_snapshots: Vec<ResponseItem> = history_snapshot
        .iter()
        .filter(|item| matches!(item, ResponseItem::GhostSnapshot { .. }))
        .cloned()
        .collect();
    new_history.extend(ghost_snapshots);

    sess.replace_history(new_history).await;
    sess.recompute_token_usage(&turn_context).await;

    let rollout_item = RolloutItem::Compacted(CompactedItem {
        message: summary_text,
        replacement_history: None,
    });
    sess.persist_rollout_items(&[rollout_item]).await;

    sess.send_event(
        &turn_context,
        EventMsg::ContextCompacted(ContextCompactedEvent {}),
    )
    .await;

    let warning = EventMsg::Warning(WarningEvent {
        message: "Heads up: Long conversations and multiple compactions can cause the model to be less accurate. Start a new conversation when possible to keep conversations small and targeted.".to_string(),
    });
    sess.send_event(&turn_context, warning).await;

    Ok(())
}

async fn apply_remote(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    replacement_history: Vec<ResponseItem>,
) -> CodexResult<()> {
    sess.replace_history(replacement_history.clone()).await;
    sess.recompute_token_usage(&turn_context).await;

    let compacted_item = CompactedItem {
        message: String::new(),
        replacement_history: Some(replacement_history),
    };
    sess.persist_rollout_items(&[RolloutItem::Compacted(compacted_item)])
        .await;

    sess.send_event(
        &turn_context,
        EventMsg::ContextCompacted(ContextCompactedEvent {}),
    )
    .await;

    Ok(())
}
