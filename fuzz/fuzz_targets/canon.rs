// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

// The clause checks live in drac-config, where the property tests drive the
// same code
// A refusal is a legitimate answer here, which is why the verdict is dropped
fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<Value>(data) else {
        return;
    };
    let _ = drac_config::conformance::assert_conforms(&value);
});
