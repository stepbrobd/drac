use clap::Args;

use crate::commands::CmdResult;

/// generated at build time (see build.rs)
const DEPS: &str = include_str!(concat!(env!("OUT_DIR"), "/deps.txt"));

/// Print build info
#[derive(Args, Debug)]
pub struct BuildInfoCmd {}

impl BuildInfoCmd {
    pub fn run(self) -> CmdResult {
        println!(
            "Binary: {} {}",
            env!("CARGO_BIN_NAME"),
            env!("CARGO_PKG_VERSION")
        );
        println!("Target: {}", env!("DRAC_TARGET"));
        println!("Profile: {}", env!("DRAC_PROFILE"));

        let deps: Vec<(&str, &str)> = DEPS.lines().filter_map(|l| l.split_once(' ')).collect();
        println!("Dependencies ({}):", deps.len());

        let width = deps.iter().map(|(name, _)| name.len()).max().unwrap_or(0);

        for (name, version) in deps {
            println!("  {name:<width$}  {version}");
        }

        Ok(())
    }
}
