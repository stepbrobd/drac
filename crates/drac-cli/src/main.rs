mod commands;

use clap::Parser;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Cli::parse().command.run()
}
