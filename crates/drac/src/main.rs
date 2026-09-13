// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // debug_assert compiles out under the release profile the checks build with
    #[test]
    fn drac_command_names_are_unique() {
        Cli::command().debug_assert();

        let names: Vec<_> = Cli::command()
            .get_subcommands()
            .map(|verb| verb.get_name().to_owned())
            .collect();
        let mut seen = names.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), names.len(), "Two subcommands share a name");
    }
}
