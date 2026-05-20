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
}
