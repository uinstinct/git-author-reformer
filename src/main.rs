use clap::Parser;
use git_author_reformer::{error, git};

#[derive(Parser)]
#[command(name = "git-author-reformer", version)]
struct Cli {}

fn run() -> Result<(), error::AppError> {
    let repo = git::open_repo()?;
    git::preflight::check_stash(&repo)?;
    git::preflight::check_worktrees(&repo)?;
    println!("git-author-reformer: preflight passed");
    Ok(())
}

fn main() {
    let _cli = Cli::parse();
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
