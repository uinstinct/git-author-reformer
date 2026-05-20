use clap::Parser;

#[derive(Parser)]
#[command(name = "git-author-reformer", version)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
