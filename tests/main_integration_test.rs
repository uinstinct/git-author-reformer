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

    assert!(
        !output.status.success(),
        "expected non-zero exit code outside a git repo"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not inside a git repository"),
        "expected 'Not inside a git repository' in stderr, got: {stderr}"
    );
}

#[test]
fn test_binary_reaches_tty_guard_when_stash_ref_exists() {
    // HOOK-12: preflight is now gated inside the TUI (Rename/Drop branches), not at startup.
    // A repo with stash entries must reach the TTY guard (not exit with a preflight error).
    let (_dir, repo) = common::create_fixture_repo();

    // Create a synthetic refs/stash pointing at HEAD — simulates a stash entry
    let head_oid = repo.head().unwrap().target().unwrap();
    repo.reference("refs/stash", head_oid, false, "stash log")
        .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))
        .current_dir(repo.workdir().unwrap())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit code (TTY guard, not stash preflight)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not an interactive terminal"),
        "expected TTY guard message (preflight is now in-TUI), got: {stderr}"
    );
    assert!(
        !stderr.contains("Stash entries detected"),
        "stash preflight must NOT fire at startup, got: {stderr}"
    );
}

#[test]
fn test_binary_reaches_tty_guard_when_linked_worktree_exists() {
    // HOOK-12: preflight is now gated inside the TUI (Rename/Drop branches), not at startup.
    // A repo with linked worktrees must reach the TTY guard (not exit with a preflight error).
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

    assert!(
        !output.status.success(),
        "expected non-zero exit code (TTY guard, not worktree preflight)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not an interactive terminal"),
        "expected TTY guard message (preflight is now in-TUI), got: {stderr}"
    );
    assert!(
        !stderr.contains("Linked worktrees detected"),
        "worktree preflight must NOT fire at startup, got: {stderr}"
    );
}

#[test]
fn test_binary_passes_preflight_on_clean_repo() {
    // Preflight passes on a clean repo; the binary then hits the TTY guard
    // (tests run without a TTY) and exits with NotATerminal — not a preflight
    // error. We verify that none of the preflight messages appear in stderr.
    let (_dir, repo) = common::create_fixture_repo();

    let output = Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))
        .current_dir(repo.workdir().unwrap())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Not inside a git repository"),
        "preflight should pass on clean repo, got: {stderr}"
    );
    assert!(
        !stderr.contains("Stash entries detected"),
        "no stash on clean repo, got: {stderr}"
    );
    assert!(
        !stderr.contains("Linked worktrees detected"),
        "no worktrees on clean repo, got: {stderr}"
    );
}

#[test]
fn test_binary_exits_cleanly_when_stdin_is_not_a_tty() {
    // Simulates `curl ... | sh`: stdin is a pipe, not a terminal.
    // The binary must detect this before ratatui::init() and exit with a
    // helpful message instead of panicking with "Failed to initialize input reader".
    let (_dir, repo) = common::create_fixture_repo();

    let output = Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))
        .current_dir(repo.workdir().unwrap())
        .stdin(std::process::Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit when stdin is not a TTY"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not an interactive terminal"),
        "expected TTY error message, got: {stderr}"
    );
    assert!(
        !stderr.contains("Failed to initialize input reader"),
        "should not see raw crossterm panic, got: {stderr}"
    );
}
