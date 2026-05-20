use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not inside a git repository: {0}")]
    NotARepo(String),

    #[error("Stash entries detected. Pop or drop all stashes before rewriting history.\nRun: git stash list")]
    StashDetected,

    #[error("Linked worktrees detected: {0}\nRemove worktrees before rewriting history.\nRun: git worktree list")]
    WorktreesDetected(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Commit {0} has a non-UTF-8 message — cannot rewrite (git2 requires valid UTF-8)")]
    NonUtf8Message(git2::Oid),
}
