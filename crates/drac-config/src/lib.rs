// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

mod canon;
mod version;

// A panicking oracle has no place in the library an embedder takes
#[cfg(any(test, feature = "fuzzing"))]
pub mod conformance;

pub use canon::{CanonError, GenerationId, canon, id};
pub use version::{CURRENT, Version, VersionError, check};
