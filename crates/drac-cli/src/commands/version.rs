use clap::Args;

use crate::commands::CmdResult;

/// Print version and exit
#[derive(Args, Debug)]
pub struct VersionCmd {}

impl VersionCmd {
    pub fn run(self) -> CmdResult {
        println!("{} {}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
        Ok(())
    }
}
