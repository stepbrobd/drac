// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

//! The re-export resolves to drac-config's own items.

use drac::config;

// A stand-in keeping the signature still accepts what drac-config refuses
#[test]
fn drac_reexports_config_from_drac_config() {
    assert_eq!(config::check(drac_config::CURRENT), Ok(()));

    let superseded: config::Version = "2026.607.0".parse().expect("Parses as calver");
    assert!(
        config::check(superseded).is_err(),
        "Accepted a version drac-config refuses"
    );
}
