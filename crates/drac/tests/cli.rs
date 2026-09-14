// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_drac"))
        .args(args)
        .output()
        .expect("Failed to run drac")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn after_prefix<'a>(stdout: &'a str, prefix: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix(prefix))
        .unwrap_or_else(|| panic!("No line starting with {prefix:?} in build-info output"))
}

#[test]
fn drac_version_prints_name_and_version() {
    let out = run(&["version"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "The version command did not exit 0"
    );
    assert_eq!(
        stdout_of(&out).trim(),
        format!("drac {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn drac_build_info_prints_build_and_locked_dependencies() {
    let out = run(&["build-info"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "The build-info command did not exit 0"
    );
    let stdout = stdout_of(&out);

    assert_eq!(
        after_prefix(&stdout, "Binary: "),
        format!("drac {}", env!("CARGO_PKG_VERSION"))
    );

    // Comparing against DRAC_TARGET would compare the build script to itself
    let host_os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let target = after_prefix(&stdout, "Target: ");
    assert!(
        target.contains(std::env::consts::ARCH) && target.contains(host_os),
        "Reported target {target:?} is not the host"
    );

    let profile = after_prefix(&stdout, "Profile: ");
    assert!(
        ["debug", "release"].contains(&profile),
        "Reported profile {profile:?} is neither debug nor release"
    );

    let count: usize = after_prefix(&stdout, "Dependencies (")
        .strip_suffix("):")
        .expect("Dependency count is not parenthesized")
        .parse()
        .expect("Dependency count is not a number");

    let deps: Vec<(&str, &str)> = stdout
        .lines()
        .filter_map(|l| l.strip_prefix("  "))
        .filter_map(|l| l.split_once("  "))
        .map(|(name, version)| (name.trim_end(), version.trim_start()))
        .collect();

    assert_eq!(
        deps.len(),
        count,
        "Listed {} dependencies under a count of {count}",
        deps.len()
    );
    // drac is a workspace member, whose version no dependency bump can move
    assert!(
        deps.contains(&("drac", env!("CARGO_PKG_VERSION"))),
        "Locked dependencies omit drac itself"
    );
    assert!(
        deps.iter().any(|(name, _)| *name == "clap"),
        "Locked dependencies omit clap"
    );
}

// The printed version parses as calver, which checks.binaries cannot assert
#[test]
fn drac_cli_version_is_calver() {
    let printed = stdout_of(&run(&["version"]));
    let shown = printed
        .trim()
        .strip_prefix("drac ")
        .expect("The version line names the binary first");
    let parsed: drac::config::Version = shown.parse().expect("The printed version is calver");
    assert_eq!(parsed.to_string(), shown);
}

#[test]
fn drac_refuses_unknown_verb() {
    // The split into two binaries retired the daemon verb
    for verb in ["nonesuch", "daemon"] {
        assert_eq!(run(&[verb]).status.code(), Some(2), "Accepted {verb}");
    }
}

#[test]
fn drac_refuses_a_bare_invocation() {
    assert_eq!(run(&[]).status.code(), Some(2), "Accepted a verbless call");
}
