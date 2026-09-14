// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

//! The clauses RFC 8785 puts on canonical output, checked over the output
//! itself rather than recomputed from the value.
//!
//! The fuzz target and the property tests both call this, which is what keeps
//! one of them from drifting into an oracle that cannot fail.

use crate::{CanonError, canon};
use serde_json::Value;
use std::iter::Peekable;
use std::str::Chars;

/// Asserts every clause the canonical output owes, for a value canon accepts.
///
/// Answers false when the value was refused, so that an encoder refusing more
/// than it should reads as an unexercised law rather than a discharged one.
pub fn assert_conforms(value: &Value) -> bool {
    let text = match canon(value) {
        Ok(text) => text,
        Err(CanonError::Number { .. } | CanonError::Depth { .. }) => return false,
    };

    let back: Value = serde_json::from_str(&text).expect("Canonical output has to reparse");
    assert_eq!(&back, value, "Canonical output changed the value");

    assert_clauses(&text);
    true
}

/// Section 3.2.1 emits no whitespace, 3.2.2.2 fixes the escape alphabet, and
/// 3.2.3 orders member names by UTF-16 code unit.
fn assert_clauses(text: &str) {
    #[derive(PartialEq)]
    enum Open {
        Object,
        Array,
    }

    let mut stack: Vec<Open> = Vec::new();
    let mut earlier: Vec<Option<Vec<u16>>> = Vec::new();
    let mut wants_name = false;
    let mut rest = text.chars().peekable();

    while let Some(c) = rest.next() {
        match c {
            '{' => {
                stack.push(Open::Object);
                earlier.push(None);
                wants_name = true;
            }
            '[' => {
                stack.push(Open::Array);
                wants_name = false;
            }
            '}' | ']' => {
                if stack.pop() == Some(Open::Object) {
                    earlier.pop();
                }
                wants_name = false;
            }
            ',' => wants_name = stack.last() == Some(&Open::Object),
            ':' => wants_name = false,
            '"' => {
                let name = take_string(&mut rest);
                if wants_name {
                    let units: Vec<u16> = name.encode_utf16().collect();
                    let slot = earlier.last_mut().expect("A name sits inside an object");
                    if let Some(before) = slot {
                        assert!(*before < units, "Members left UTF-16 order at {name:?}");
                    }
                    *slot = Some(units);
                }
                wants_name = false;
            }
            c if c.is_ascii_whitespace() => panic!("Canonical output carries whitespace"),
            _ => {}
        }
    }
    assert!(stack.is_empty(), "Canonical output left a container open");
}

/// Consumes one string body past its opening quote, refusing any escape the
/// RFC does not name and any scalar that took an escape it does not owe.
fn take_string(rest: &mut Peekable<Chars<'_>>) -> String {
    let mut decoded = String::new();
    loop {
        let c = rest.next().expect("A string has a closing quote");
        match c {
            '"' => return decoded,
            '\\' => {
                let marker = rest.next().expect("An escape has a body");
                decoded.push(match marker {
                    'b' => '\u{8}',
                    't' => '\u{9}',
                    'n' => '\u{a}',
                    'f' => '\u{c}',
                    'r' => '\u{d}',
                    '"' => '"',
                    '\\' => '\\',
                    'u' => take_hex(rest),
                    other => panic!("Escape \\{other} is outside the RFC alphabet"),
                });
            }
            c => {
                assert!(c as u32 >= 0x20, "An unescaped control character");
                decoded.push(c);
            }
        }
    }
}

fn take_hex(rest: &mut Peekable<Chars<'_>>) -> char {
    let mut raw = 0u32;
    for _ in 0..4 {
        let digit = rest.next().expect("A \\u escape carries four digits");
        assert!(
            digit.is_ascii_digit() || ('a'..='f').contains(&digit),
            "Escape hex is not lowercase"
        );
        raw = raw * 16 + digit.to_digit(16).expect("Checked one line above");
    }
    assert!(raw < 0x20, "U+{raw:04X} took a \\u escape it does not owe");
    assert!(
        !matches!(raw, 0x8 | 0x9 | 0xa | 0xc | 0xd),
        "U+{raw:04X} has a two character escape and took \\u"
    );
    char::from_u32(raw).expect("Below U+0020 by the assertion above")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // The oracle is the fuzz target's whole verdict, and an oracle that stops
    // asserting reads exactly like one that finds nothing
    // Each case below is output a conforming encoder cannot produce

    #[test]
    fn drac_config_conformance_answers_false_for_a_refused_value() {
        assert!(!assert_conforms(&json!(1.5)));
        assert!(!assert_conforms(&json!({ "ttl": 0.5 })));
        assert!(assert_conforms(&json!({ "a": 1 })));
    }

    #[test]
    #[should_panic(expected = "carries whitespace")]
    fn drac_config_conformance_refuses_whitespace_between_tokens() {
        assert_clauses("{\"a\": 1}");
    }

    #[test]
    #[should_panic(expected = "not lowercase")]
    fn drac_config_conformance_refuses_uppercase_escape_hex() {
        assert_clauses("\"\\u001F\"");
    }

    #[test]
    #[should_panic(expected = "has a two character escape")]
    fn drac_config_conformance_refuses_a_long_escape_for_a_short_one() {
        assert_clauses("\"\\u0009\"");
    }

    #[test]
    #[should_panic(expected = "does not owe")]
    fn drac_config_conformance_refuses_an_escape_the_rfc_leaves_literal() {
        assert_clauses("\"\\u007f\"");
    }

    #[test]
    #[should_panic(expected = "outside the RFC alphabet")]
    fn drac_config_conformance_refuses_an_escaped_solidus() {
        assert_clauses("\"\\/\"");
    }

    #[test]
    #[should_panic(expected = "left UTF-16 order")]
    fn drac_config_conformance_refuses_members_out_of_order() {
        assert_clauses("{\"b\":1,\"a\":2}");
    }

    #[test]
    #[should_panic(expected = "left UTF-16 order")]
    fn drac_config_conformance_refuses_utf8_order_at_the_astral_boundary() {
        // UTF-8 byte order puts the astral key last, UTF-16 puts it first
        assert_clauses("{\"\u{fb33}\":1,\"\u{1f602}\":2}");
    }

    #[test]
    fn drac_config_conformance_accepts_nested_and_astral_output() {
        assert_clauses("{\"a\":{\"b\":[1,{\"c\":\"x\"}]},\"\u{1f602}\":\"y\"}");
    }
}
