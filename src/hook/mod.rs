pub mod parse;
pub mod path;
pub mod render;
pub mod write;

use std::path::PathBuf;

pub enum HookState {
    Absent,
    Managed { emails: Vec<String> },
    NotToolManaged(PathBuf),
}

pub enum AddResult {
    Installed { count: usize },
    AlreadyStripped,
}

pub enum RemoveResult {
    Updated { remaining: usize },
    HookDeleted,
    NotFound,
}

pub fn install_strip(
    _repo: &git2::Repository,
    _email: &str,
) -> Result<AddResult, crate::error::AppError> {
    unimplemented!("Plan 04 wires this")
}

pub fn remove_strip(
    _repo: &git2::Repository,
    _email: &str,
) -> Result<RemoveResult, crate::error::AppError> {
    unimplemented!("Plan 04 wires this")
}

pub fn read_strip_list(
    _repo: &git2::Repository,
) -> Result<HookState, crate::error::AppError> {
    unimplemented!("Plan 02 wires this")
}
