pub fn check_stash(repo: &git2::Repository) -> Result<(), crate::error::AppError> {
    if repo.find_reference("refs/stash").is_ok() {
        return Err(crate::error::AppError::StashDetected);
    }
    Ok(())
}

pub fn check_worktrees(repo: &git2::Repository) -> Result<(), crate::error::AppError> {
    let worktrees = repo.worktrees()?;
    if !worktrees.is_empty() {
        let names: Vec<&str> = worktrees.iter().filter_map(|r| r.ok().flatten()).collect();
        return Err(crate::error::AppError::WorktreesDetected(names.join(", ")));
    }
    Ok(())
}
