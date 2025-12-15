use std::collections::BTreeSet;
use std::path::Path;

use sha1::Digest;
use tokio::time::timeout;

const MAX_FILES: usize = 5;
const GIT_COMMAND_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorktreeSnapshot {
    pub(crate) dirty_paths: Vec<String>,
    pub(crate) fingerprint: String,
}

pub(crate) async fn snapshot_dirty_paths(repo_root: &Path) -> Option<WorktreeSnapshot> {
    let output = timeout(
        GIT_COMMAND_TIMEOUT,
        tokio::process::Command::new("git")
            .args(["status", "--porcelain=2", "-z"])
            .current_dir(repo_root)
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    if !output.status.success() {
        return None;
    }

    let mut paths = parse_porcelain_v2_z_paths(&output.stdout, MAX_FILES + 1);
    paths.sort();

    let fingerprint = fingerprint_paths(&paths);

    Some(WorktreeSnapshot {
        dirty_paths: paths,
        fingerprint,
    })
}

fn fingerprint_paths(paths: &[String]) -> String {
    let joined = paths.join("\n");
    let digest = sha1::Sha1::digest(joined.as_bytes());
    format!("{digest:x}")
}

pub(crate) fn compute_external_changed_paths(
    baseline: &WorktreeSnapshot,
    current: &WorktreeSnapshot,
    last_turn_codex_touched_paths: &[String],
) -> Vec<String> {
    let mut baseline_set: BTreeSet<&str> =
        baseline.dirty_paths.iter().map(String::as_str).collect();
    for path in last_turn_codex_touched_paths {
        baseline_set.insert(path.as_str());
    }

    current
        .dirty_paths
        .iter()
        .filter(|path| !baseline_set.contains(path.as_str()))
        .cloned()
        .collect()
}

pub(crate) fn compute_external_changed_paths_unattributed(
    baseline: &WorktreeSnapshot,
    current: &WorktreeSnapshot,
) -> Vec<String> {
    compute_external_changed_paths(baseline, current, &[])
}

pub(crate) fn should_warn_external_change(
    baseline: &WorktreeSnapshot,
    current: &WorktreeSnapshot,
    external_paths: &[String],
) -> bool {
    if !external_paths.is_empty() {
        return true;
    }

    // Detect "changed but now clean" or other churn that results in no current dirty paths.
    // If the set fingerprint changed and we used to have dirty paths, we still warn.
    baseline.fingerprint != current.fingerprint && !baseline.dirty_paths.is_empty()
}

pub(crate) fn format_warning_message(external_paths: &[String]) -> String {
    let mut message =
        "Working tree changed outside this session since last turn. If this was intentional, tell Codex what changed."
            .to_string();

    if external_paths.is_empty() {
        return message;
    }

    let mut display: Vec<String> = external_paths.to_vec();
    display.sort();
    if display.len() > MAX_FILES {
        display.truncate(MAX_FILES);
        display[MAX_FILES - 1] = "...".to_string();
    }

    for path in display {
        message.push('\n');
        message.push_str("- ");
        message.push_str(&path);
    }

    message
}

fn parse_porcelain_v2_z_paths(input: &[u8], limit: usize) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for entry in input.split(|b| *b == 0) {
        if entry.is_empty() {
            continue;
        }
        if let Some(path) = parse_porcelain_v2_entry_path(entry) {
            paths.push(path);
            if paths.len() >= limit {
                break;
            }
        }
    }
    paths
}

fn parse_porcelain_v2_entry_path(entry: &[u8]) -> Option<String> {
    // Relevant entries we care about (porcelain v2):
    // `1 <xy> ... <path>`
    // `2 <xy> ... <score> <path>\0<orig_path>`
    // `? <path>` (untracked)
    // `! <path>` (ignored)
    // We'll ignore ignored, but include untracked.

    let s = std::str::from_utf8(entry).ok()?;
    let mut chars = s.chars();
    let first = chars.next()?;
    match first {
        '?' => Some(s.get(2..)?.to_string()),
        '1' => s.rsplit_once(' ').map(|(_, p)| p.to_string()),
        '2' => {
            // For renames, the entry includes NUL-separated old path, but we split on NUL already,
            // so we only see the first segment containing the new path.
            s.rsplit_once(' ').map(|(_, p)| p.to_string())
        }
        '!' => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_porcelain_v2_z_extracts_paths() {
        let input = b"1 .M N... 100644 100644 100644 abcdef0 abcdef1 file_a\0? file_b\0! file_c\0";
        let paths = parse_porcelain_v2_z_paths(input, 10);
        assert_eq!(paths, vec!["file_a".to_string(), "file_b".to_string()]);
    }

    #[test]
    fn format_warning_message_truncates_and_uses_ellipsis() {
        let paths = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
            "f".to_string(),
        ];
        let msg = format_warning_message(&paths);
        assert!(msg.contains("- a"));
        assert!(msg.contains("- b"));
        assert!(msg.contains("- c"));
        assert!(msg.contains("- d"));
        assert!(msg.contains("- ..."));
        assert!(!msg.contains("- e"));
        assert!(!msg.contains("- f"));
    }

    #[test]
    fn compute_external_changed_paths_subtracts_baseline_and_codex_touched() {
        let baseline = WorktreeSnapshot {
            dirty_paths: vec!["a".to_string(), "b".to_string()],
            fingerprint: "x".to_string(),
        };
        let current = WorktreeSnapshot {
            dirty_paths: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            fingerprint: "y".to_string(),
        };

        let external = compute_external_changed_paths(&baseline, &current, &["c".to_string()]);
        assert!(external.is_empty());

        let external = compute_external_changed_paths(&baseline, &current, &[]);
        assert_eq!(external, vec!["c".to_string()]);
    }

    #[test]
    fn should_warn_for_fingerprint_change_when_now_clean() {
        let baseline = WorktreeSnapshot {
            dirty_paths: vec!["a".to_string()],
            fingerprint: "old".to_string(),
        };
        let current = WorktreeSnapshot {
            dirty_paths: vec![],
            fingerprint: "new".to_string(),
        };

        assert!(should_warn_external_change(&baseline, &current, &[]));
    }

    // Config opt-out is handled at the call sites that decide whether to emit the warning.
}
