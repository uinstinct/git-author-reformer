pub mod preflight;
pub mod reader;
pub mod types;

pub fn open_repo() -> Result<git2::Repository, crate::error::AppError> {
    git2::Repository::open_from_env().map_err(|e| crate::error::AppError::NotARepo(e.to_string()))
}
