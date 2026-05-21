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

    #[error("Not an interactive terminal.\ngit-author-reformer is a TUI application — run it directly, not inside a pipe.\nExample: git-author-reformer")]
    NotATerminal,

    #[error("Terminal I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Existing commit-msg hook at {0:?} is not managed by git-author-reformer.\nRemove or rename the file, then re-run.")]
    HookExists(std::path::PathBuf),

    #[error("Email {email:?} contains a character not allowed in the hook strip list (forbidden: {forbidden_char:?}).")]
    HookInvalidEmail { email: String, forbidden_char: char },
}
