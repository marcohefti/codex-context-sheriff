use crate::worktree_change_notice::WorktreeSnapshot;

fn should_pause(
    bypass_once: bool,
    baseline: &WorktreeSnapshot,
    current: &WorktreeSnapshot,
) -> bool {
    if bypass_once {
        return false;
    }

    let external = crate::worktree_change_notice::compute_external_changed_paths_unattributed(
        baseline, current,
    );

    crate::worktree_change_notice::should_warn_external_change(baseline, current, &external)
}

#[test]
fn soft_pause_triggers_when_external_change_detected() {
    let baseline = WorktreeSnapshot {
        dirty_paths: vec!["a".to_string()],
        fingerprint: "old".to_string(),
    };
    let current = WorktreeSnapshot {
        dirty_paths: vec!["a".to_string(), "README.md".to_string()],
        fingerprint: "new".to_string(),
    };

    assert!(should_pause(false, &baseline, &current));
}

#[test]
fn soft_pause_is_suppressed_once_when_bypassed() {
    let baseline = WorktreeSnapshot {
        dirty_paths: vec!["a".to_string()],
        fingerprint: "old".to_string(),
    };
    let current = WorktreeSnapshot {
        dirty_paths: vec!["a".to_string(), "README.md".to_string()],
        fingerprint: "new".to_string(),
    };

    assert!(!should_pause(true, &baseline, &current));
    assert!(should_pause(false, &baseline, &current));
}
