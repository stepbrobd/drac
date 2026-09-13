// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

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
