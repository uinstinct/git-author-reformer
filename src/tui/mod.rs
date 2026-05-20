pub mod app;
pub mod event;
pub mod render;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use ratatui::DefaultTerminal;

pub fn run_with_terminal(
    _terminal: &mut DefaultTerminal,
    _repo: git2::Repository,
    _term_flag: Arc<AtomicBool>,
) -> Result<(), crate::error::AppError> {
    // Stub — implemented in GREEN step of Task 2
    Ok(())
}
