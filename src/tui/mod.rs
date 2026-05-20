pub mod app;
pub mod event;
pub mod render;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crossterm::event::{Event, KeyEventKind};
use ratatui::DefaultTerminal;

pub fn run_with_terminal(
    terminal: &mut DefaultTerminal,
    repo: git2::Repository,
    term_flag: Arc<AtomicBool>,
) -> Result<(), crate::error::AppError> {
    let mut app = app::App::new(repo);
    loop {
        if term_flag.load(Ordering::Relaxed) {
            break;
        }
        terminal.draw(|f| render::render(f, &app))?;
        if crossterm::event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind == KeyEventKind::Press {
                    event::handle_key(&mut app, key.code);
                }
            }
        }
        if app.should_exit {
            break;
        }
    }
    Ok(())
}
