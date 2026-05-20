mod common;

use std::process::Command;

#[test]
fn test_binary_exits_with_error_outside_git_repo() {
    let dir = tempfile::TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))
        .current_dir(dir.path())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .output()
        .unwrap();

    assert!(!output.status.success(), "expected non-zero exit code outside a git repo");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not inside a git repository"),
        "expected 'Not inside a git repository' in stderr, got: {stderr}"
    );
}

#[test]
fn test_binary_blocks_when_stash_ref_exists() {
    let (_dir, repo) = common::create_fixture_repo();

    // Create a synthetic refs/stash pointing at HEAD — simulates a stash entry
    let head_oid = repo.head().unwrap().target().unwrap();
    repo.reference("refs/stash", head_oid, false, "stash log").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))
        .current_dir(repo.workdir().unwrap())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .output()
        .unwrap();

    assert!(!output.status.success(), "expected non-zero exit code with stash ref");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Stash entries detected"),
        "expected 'Stash entries detected' in stderr, got: {stderr}"
    );
}

#[test]
fn test_binary_blocks_when_linked_worktree_exists() {
    let (_dir, repo) = common::create_fixture_repo();

    // Create a sibling TempDir; pass a non-existent sub-path as the worktree location
    // (git2 worktree() requires the path to NOT exist yet — it creates it)
    let wt_parent = tempfile::TempDir::new().unwrap();
    let wt_path = wt_parent.path().join("linked-wt");

    repo.worktree("test-wt", &wt_path, None).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))
        .current_dir(repo.workdir().unwrap())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .output()
        .unwrap();

    assert!(!output.status.success(), "expected non-zero exit code with linked worktree");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Linked worktrees detected"),
        "expected 'Linked worktrees detected' in stderr, got: {stderr}"
    );
}

#[test]
fn test_binary_passes_preflight_on_clean_repo() {
    let (_dir, repo) = common::create_fixture_repo();

    let output = Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))
        .current_dir(repo.workdir().unwrap())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .output()
        .unwrap();

    assert!(output.status.success(), "expected exit code 0 on clean repo");
}
