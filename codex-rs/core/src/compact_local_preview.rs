use std::sync::Arc;

use crate::codex::Session;
use crate::codex::TurnContext;
use crate::error::Result as CodexResult;
use codex_protocol::user_input::UserInput;

use crate::compact_preview::CompactionPreview;
use crate::compact_preview::generate_local_preview;

pub(crate) async fn run_local_compact_preview(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    input: Vec<UserInput>,
) -> CodexResult<CompactionPreview> {
    let output =
        generate_local_preview(Arc::clone(&sess), Arc::clone(&turn_context), input).await?;
    Ok(CompactionPreview::Local {
        summary_text: output.summary_text,
        carry_over: output.carry_over,
    })
}
