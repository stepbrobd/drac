use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let lock_path =
        find_cargo_lock(&manifest_dir).expect("no Cargo.lock found above CARGO_MANIFEST_DIR");

    // regenerate only when the lockfile changes
    println!("cargo::rerun-if-changed={}", lock_path.display());

    // forward target and profile
    println!(
        "cargo::rustc-env=DRAC_TARGET={}",
        env::var("TARGET").unwrap_or_default()
    );
    println!(
        "cargo::rustc-env=DRAC_PROFILE={}",
        env::var("PROFILE").unwrap_or_default()
    );

    let lock = fs::read_to_string(&lock_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", lock_path.display()));

    let mut deps = parse_locked_dependencies(&lock);
    deps.sort();

    let body: String = deps.iter().map(|(n, v)| format!("{n} {v}\n")).collect();
    let dest = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set")).join("deps.txt");
    fs::write(&dest, body).unwrap_or_else(|e| panic!("failed to write {}: {e}", dest.display()));
}

fn find_cargo_lock(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join("Cargo.lock"))
        .find(|candidate| candidate.is_file())
}

fn parse_locked_dependencies(lock: &str) -> Vec<(String, String)> {
    let mut deps = Vec::new();
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;

    for line in lock.lines().map(str::trim) {
        if line.starts_with('[') {
            if let (Some(n), Some(v)) = (name.take(), version.take()) {
                deps.push((n, v));
            }
        } else if let Some(v) = value_of(line, "name") {
            name = Some(v);
        } else if let Some(v) = value_of(line, "version") {
            version = Some(v);
        }
    }

    if let (Some(n), Some(v)) = (name, version) {
        deps.push((n, v));
    }

    deps
}

fn value_of(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    Some(rest.strip_prefix('"')?.strip_suffix('"')?.to_string())
}
