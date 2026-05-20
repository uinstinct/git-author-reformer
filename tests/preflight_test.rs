mod common;

use git_author_reformer::error::AppError;
use git_author_reformer::git::preflight::{check_stash, check_worktrees};

/// Stash detection: clean repo (no refs/stash) must pass.
/// check_stash exists to block rewrites that would orphan the stash ref.
/// If no stash exists, the gate must be transparent.
#[test]
fn test_check_stash_passes_on_clean_repo() {
    let (_dir, repo) = common::create_fixture_repo();
    let result = check_stash(&repo);
    assert!(
        result.is_ok(),
        "clean repo should pass stash gate; got: {result:?}"
    );
}

/// Stash detection: repo with refs/stash must be blocked (SAFE-01).
/// A stash entry would be orphaned after a history rewrite.
#[test]
fn test_check_stash_blocks_when_stash_ref_exists() {
    let (_dir, repo) = common::create_fixture_repo();
    let head_oid = repo.head().unwrap().peel_to_commit().unwrap().id();
    repo.reference("refs/stash", head_oid, false, "test stash")
        .unwrap();

    let result = check_stash(&repo);
    assert!(
        matches!(result, Err(AppError::StashDetected)),
        "repo with refs/stash must return Err(StashDetected); got: {result:?}"
    );
}

/// Worktree detection (Pitfall 4): main worktree only — no linked worktrees.
/// libgit2's repo.worktrees() excludes the main worktree; an empty result means safe.
#[test]
fn test_check_worktrees_passes_on_single_worktree_repo() {
    let (_dir, repo) = common::create_fixture_repo();
    let result = check_worktrees(&repo);
    assert!(
        result.is_ok(),
        "repo with only main worktree should pass worktree gate; got: {result:?}"
    );
}

/// Worktree detection: linked worktree present must be blocked (SAFE-02).
/// Linked worktrees hold locks on their checked-out branches; the rewrite
/// cannot update those branches without removing the worktree first.
#[test]
fn test_check_worktrees_blocks_when_linked_worktree_exists() {
    let (_dir, repo) = common::create_fixture_repo();
    // Linked worktree path must be outside the main repo dir (libgit2 requirement).
    // Use a non-existent subdirectory — libgit2 creates it; passing an existing dir fails.
    let wt_parent = tempfile::TempDir::new().unwrap();
    let wt_path = wt_parent.path().join("linked-wt");
    repo.worktree("test-wt", &wt_path, None).unwrap();

    let result = check_worktrees(&repo);
    assert!(
        matches!(result, Err(AppError::WorktreesDetected(_))),
        "repo with a linked worktree must return Err(WorktreesDetected); got: {result:?}"
    );
}
