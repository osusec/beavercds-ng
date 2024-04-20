// I think we need this?
use clap::Parser;

mod commands;

fn main() {
    let _cli = commands::Cli::parse();
}
