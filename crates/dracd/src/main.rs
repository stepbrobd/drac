// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

//! The drac daemon.
//!
//! Versioned configuration is its only input.

use std::process::ExitCode;

/// The usage exit code clap gives the CLI.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    if let Some(arg) = std::env::args_os().nth(1) {
        eprintln!("dracd takes no arguments, got {arg:?}");
        return ExitCode::from(EXIT_USAGE);
    }
    ExitCode::SUCCESS
}
