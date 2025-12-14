use crate::compact_preview::CompactionPreview;

pub(crate) fn render_compaction_preview(preview: &CompactionPreview) -> String {
    match preview {
        CompactionPreview::Local {
            summary_text,
            carry_over,
        } => render(
            summary_text,
            carry_over.percent(),
            carry_over.was_truncated,
            false,
        ),
        CompactionPreview::Remote {
            replacement_history,
            carry_over,
        } => {
            let summary_text =
                crate::compact_preview::extract_remote_summary_text(replacement_history);
            render(
                &summary_text,
                carry_over.percent(),
                carry_over.was_truncated,
                true,
            )
        }
    }
}

fn render(summary_text: &str, percent: u64, was_truncated: bool, is_remote: bool) -> String {
    let label = if is_remote {
        "(compaction preview - remote)"
    } else {
        "(compaction preview)"
    };

    let trunc_note = if was_truncated {
        " (last message truncated)"
    } else {
        ""
    };

    format!(
        "{label}\n\n{summary_text}\n\nRetained recent user messages: {percent}%{trunc_note}\n\nApply this preview with /compact --apply"
    )
}
