use clap::Parser;
use git_author_reformer::{error, git, tui};
use std::io::IsTerminal;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "git-author-reformer", version)]
struct Cli {}

fn run() -> Result<(), error::AppError> {
    // 1. SIGTERM flag — registered BEFORE ratatui::init() (RESEARCH §Pattern 1).
    let term_flag = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term_flag))?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term_flag))?;
    signal_hook::flag::register(signal_hook::consts::SIGHUP, Arc::clone(&term_flag))?;

    // 2. Pre-flight (existing Phase 1 gates).
    let repo = git::open_repo()?;
    git::preflight::check_stash(&repo)?;
    git::preflight::check_worktrees(&repo)?;

    // 3. TTY guard — must run after pre-flight so those errors still surface
    //    cleanly, but before ratatui::init() which panics on a non-TTY stdin.
    //    Triggered when the binary is invoked via `curl ... | sh`.
    if !std::io::stdin().is_terminal() {
        return Err(error::AppError::NotATerminal);
    }

    // 4. ratatui::init() installs panic hook + raw mode + alternate screen.
    let mut terminal = ratatui::init();

    // 5. Run the TUI; capture result so we can ALWAYS call restore().
    let result = tui::run_with_terminal(&mut terminal, repo, term_flag);

    // 6. Restore on EVERY exit path — happy and error.
    ratatui::restore();

    // 7. Propagate after restore so terminal is clean before printing.
    result
}

fn main() {
    let _cli = Cli::parse();
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
