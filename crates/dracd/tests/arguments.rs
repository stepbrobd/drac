// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dracd"))
        .args(args)
        .output()
        .expect("Failed to run dracd")
}

#[test]
fn dracd_accepts_bare_invocation() {
    let out = run(&[]);
    assert_eq!(out.status.code(), Some(0), "Refused a bare invocation");
    assert!(out.stderr.is_empty(), "Wrote to stderr on a clean run");
}

#[test]
fn dracd_refuses_every_argument() {
    for arg in ["--help", "--version", "-v", "daemon", "/etc/drac/lab.toml"] {
        let out = run(&[arg]);
        assert_eq!(out.status.code(), Some(2), "Accepted {arg}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("no arguments"),
            "Refused {arg} without saying why"
        );
        assert!(stderr.contains(arg), "Refused {arg} without naming it");
    }
}
