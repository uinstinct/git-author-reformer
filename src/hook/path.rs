use std::path::PathBuf;

pub(crate) fn commit_msg_hook_path(repo: &git2::Repository) -> PathBuf {
    repo.path().join("hooks").join("commit-msg")
}
