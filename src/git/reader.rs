pub fn enumerate_authors(
    _repo: &git2::Repository,
) -> Result<Vec<crate::git::types::AuthorIdentity>, crate::error::AppError> {
    todo!("implemented in Plan 03")
}

pub fn enumerate_coauthors(
    _repo: &git2::Repository,
) -> Result<Vec<crate::git::types::CoAuthorEntry>, crate::error::AppError> {
    todo!("implemented in Plan 03")
}
