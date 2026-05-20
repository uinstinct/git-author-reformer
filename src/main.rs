use clap::Parser;
use git_author_reformer::{error, git, tui};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Parser)]
#[command(name = "git-author-reformer", version)]
struct Cli {}

fn run() -> Result<(), error::AppError> {
    // 1. SIGTERM flag — registered BEFORE ratatui::init() (RESEARCH §Pattern 1).
    let term_flag = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term_flag))?;

    // 2. Pre-flight (existing Phase 1 gates).
    let repo = git::open_repo()?;
    git::preflight::check_stash(&repo)?;
    git::preflight::check_worktrees(&repo)?;

    // 3. ratatui::init() installs panic hook + raw mode + alternate screen.
    let mut terminal = ratatui::init();

    // 4. Run the TUI; capture result so we can ALWAYS call restore().
    let result = tui::run_with_terminal(&mut terminal, repo, term_flag);

    // 5. Restore on EVERY exit path — happy and error.
    ratatui::restore();

    // 6. Propagate after restore so terminal is clean before printing.
    result
}

fn main() {
    let _cli = Cli::parse();
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
