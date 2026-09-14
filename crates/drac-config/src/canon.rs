// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

use serde_json::{Map, Number, Value};
use std::cmp::Ordering;
use std::fmt;
use thiserror::Error;

/// Rejection reasons for canonical encoding.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CanonError {
    /// A number that is not an integer in the range v0 accepts.
    /// The value shown is how serde_json reads the literal, not the literal.
    #[error(
        "Number at {} reads as {value}, not an integer between -2^53 and 2^53",
        as_json_string(pointer)
    )]
    Number { pointer: String, value: String },
    /// A value nested deeper than this crate can read back.
    #[error("Value at {} nests past {limit} containers", as_json_string(pointer))]
    Depth { pointer: String, limit: usize },
}

/// A double holds every integer up to 2^53, and past it only some.
/// Refusing the whole interval is the rule an operator can predict.
const MAX_CONTIGUOUS_INT: i64 = 1 << 53;

/// serde_json reads back at most this many nested containers.
/// Output this crate cannot read is worse than a refusal.
const MAX_DEPTH: usize = 127;

/// A generation id, blake3 over the canonical encoding of the desired state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenerationId(blake3::Hash);

impl fmt::Display for GenerationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.to_hex().as_str())
    }
}

/// The integers v0 accepts, which `number` and the proof below both read.
fn exact(i: i64) -> bool {
    (-MAX_CONTIGUOUS_INT..=MAX_CONTIGUOUS_INT).contains(&i)
}

/// Renders a pointer as a JSON string, since RFC 6901 admits any character in a key.
/// This shares the canonical escaper, which means widening that table to make a
/// diagnostic readable would move every generation id.
fn as_json_string(pointer: &str) -> String {
    let mut out = String::new();
    string(pointer, &mut out);
    out
}

/// Encodes a value as canonical JSON per RFC 8785, over the subset v0 accepts.
///
/// Floats, non-finite numbers, integers outside [-2^53, 2^53] and nesting past
/// 127 containers are refused. A duplicate property name cannot be seen from
/// here, because a parsed value cannot carry one. The amendment to DR-4 in the
/// design records all four.
pub fn canon(value: &Value) -> Result<String, CanonError> {
    let mut out = String::new();
    let mut pointer = String::new();
    encode(value, &mut pointer, 0, &mut out)?;
    Ok(out)
}

/// Hashes the canonical encoding of a value into its generation id.
pub fn id(value: &Value) -> Result<GenerationId, CanonError> {
    Ok(GenerationId(blake3::hash(canon(value)?.as_bytes())))
}

fn encode(
    value: &Value,
    pointer: &mut String,
    depth: usize,
    out: &mut String,
) -> Result<(), CanonError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => number(n, pointer, out)?,
        Value::String(s) => string(s, out),
        Value::Array(items) => {
            check_depth(pointer, depth)?;
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                let mark = pointer.len();
                pointer.push('/');
                pointer.push_str(&index.to_string());
                encode(item, pointer, depth + 1, out)?;
                pointer.truncate(mark);
            }
            out.push(']');
        }
        Value::Object(members) => {
            check_depth(pointer, depth)?;
            object(members, pointer, depth, out)?;
        }
    }
    Ok(())
}

fn number(n: &Number, pointer: &str, out: &mut String) -> Result<(), CanonError> {
    // Accepting a known shape rather than rejecting one keeps a number
    // serde_json cannot classify from reaching the output
    if !n.as_i64().is_some_and(exact) {
        return Err(CanonError::Number {
            pointer: pointer.to_string(),
            value: n.to_string(),
        });
    }
    out.push_str(&n.to_string());
    Ok(())
}

fn object(
    members: &Map<String, Value>,
    pointer: &mut String,
    depth: usize,
    out: &mut String,
) -> Result<(), CanonError> {
    let mut sorted: Vec<(&String, &Value)> = members.iter().collect();
    sorted.sort_unstable_by(|(a, _), (b, _)| utf16_order(a, b));

    out.push('{');
    for (index, (key, value)) in sorted.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        string(key, out);
        out.push(':');

        let mark = pointer.len();
        pointer.push('/');
        segment(key, pointer);
        encode(value, pointer, depth + 1, out)?;
        pointer.truncate(mark);
    }
    out.push('}');
    Ok(())
}

/// RFC 6901 spells a key with ~0 and ~1 where it carries a tilde or a solidus.
fn segment(key: &str, pointer: &mut String) {
    for c in key.chars() {
        match c {
            '~' => pointer.push_str("~0"),
            '/' => pointer.push_str("~1"),
            c => pointer.push(c),
        }
    }
}

/// Counting containers rather than values keeps the limit off the innermost leaf.
fn check_depth(pointer: &str, depth: usize) -> Result<(), CanonError> {
    if depth >= MAX_DEPTH {
        return Err(CanonError::Depth {
            pointer: pointer.to_string(),
            limit: MAX_DEPTH,
        });
    }
    Ok(())
}

/// RFC 8785 section 3.2.3 orders members by UTF-16 code unit.
/// UTF-8 byte order disagrees for an astral character against U+E000 and above.
/// The proof below stays tractable only while this allocates nothing.
fn utf16_order(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        escape(c, out);
    }
    out.push('"');
}

/// RFC 8785 section 3.2.2.2, which is JSON's minimal escape set.
/// U+007F and the solidus stay literal, where the permissive set escapes both.
fn escape(c: char, out: &mut String) {
    // A nibble table rather than write!, which keeps this out of core::fmt
    const HEX: [u8; 16] = *b"0123456789abcdef";
    match c {
        '\u{8}' => out.push_str("\\b"),
        '\u{9}' => out.push_str("\\t"),
        '\u{a}' => out.push_str("\\n"),
        '\u{c}' => out.push_str("\\f"),
        '\u{d}' => out.push_str("\\r"),
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        c if (c as u32) < 0x20 => {
            let n = c as u32;
            out.push_str("\\u00");
            out.push(HEX[(n >> 4) as usize] as char);
            out.push(HEX[(n & 0xf) as usize] as char);
        }
        c => out.push(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    // A document shaped like design section 5, reaching the escape set, both
    // sides of the astral boundary, the integer bound and nested containers
    // The bytes and the id are written out rather than computed
    // A change to the canonical output moves them instead of moving with them
    fn golden() -> Value {
        json!({
            "version": "2026.610.0",
            "role": { "voter": ["butte", "feldberg"], "announcer": ["isere"] },
            "dns": {
                "bind": "::",
                "port": 53,
                "catalog": ["ysun.co"],
                "zone": { "ysun.co": { "records": { "@": [{ "type": "A", "ttl": 300 }] } } }
            },
            "logging": { "level": "info" },
            "notes": {
                "quote\"back\\slash": "tab\ttab",
                "\u{1f602}": "astral",
                "\u{fb33}": "bmp",
                "ctl\u{b}": "vertical",
                "solidus/": "kept",
                "del\u{7f}": "kept"
            },
            "bounds": [9007199254740992i64, -9007199254740992i64, 0]
        })
    }

    const GOLDEN_BYTES: &str = "{\"bounds\":[9007199254740992,-9007199254740992,0],\"dns\":{\"bind\":\"::\",\"catalog\":[\"ysun.co\"],\"port\":53,\"zone\":{\"ysun.co\":{\"records\":{\"@\":[{\"ttl\":300,\"type\":\"A\"}]}}}},\"logging\":{\"level\":\"info\"},\"notes\":{\"ctl\\u000b\":\"vertical\",\"del\u{7f}\":\"kept\",\"quote\\\"back\\\\slash\":\"tab\\ttab\",\"solidus/\":\"kept\",\"\u{1f602}\":\"astral\",\"\u{fb33}\":\"bmp\"},\"role\":{\"announcer\":[\"isere\"],\"voter\":[\"butte\",\"feldberg\"]},\"version\":\"2026.610.0\"}";

    const GOLDEN_ID: &str = "7b8f0585f71cfeebf41954acf932872cb36188af6580fde59e7e2334a662545b";

    #[test]
    fn drac_config_canon_matches_the_golden_document() {
        let text = canon(&golden()).expect("The golden document canonicalizes");
        assert_eq!(text, GOLDEN_BYTES);
        assert_eq!(
            id(&golden())
                .expect("The golden document hashes")
                .to_string(),
            GOLDEN_ID
        );
    }

    // Only drac_config_canon_matches_the_golden_document pins the separators
    // Regenerating GOLDEN_BYTES from the code would retire that silently
    // A canon that refuses more than it should reads as an unexercised law,
    // which is what the verdict here rules out
    #[test]
    fn drac_config_canon_conforms_over_the_golden_document() {
        assert!(crate::conformance::assert_conforms(&golden()));
    }

    #[test]
    fn drac_config_canon_emits_no_whitespace() {
        let value = json!({ "b": [1, 2], "a": { "c": true } });
        let text = canon(&value).expect("Canonicalizes");
        assert_eq!(text, "{\"a\":{\"c\":true},\"b\":[1,2]}");
    }

    #[test]
    fn drac_config_id_is_the_same_for_two_equal_values() {
        // assert_ne alone leaves an equality that always answers false green
        assert_eq!(
            id(&golden()).expect("Hashes"),
            id(&golden()).expect("Hashes")
        );
    }

    #[test]
    fn drac_config_id_refuses_what_canon_refuses() {
        assert!(matches!(id(&json!(1.5)), Err(CanonError::Number { .. })));
    }

    #[test]
    fn drac_config_canon_pops_a_member_off_the_pointer() {
        // A pointer that never truncates reads /a/b/z rather than /z
        let value = json!({ "a": { "b": 1 }, "z": 0.5 });
        let Err(CanonError::Number { pointer, .. }) = canon(&value) else {
            panic!("A fractional number has to be refused");
        };
        assert_eq!(pointer, "/z");
    }

    // An operator reads these, and both refusals are otherwise pinned only by
    // their fields, which a message rewrite leaves untouched
    #[test]
    fn drac_config_canon_says_what_it_refused_and_where() {
        let refused = canon(&json!({ "ttl": 0.5 })).expect_err("Refused");
        assert_eq!(
            refused.to_string(),
            "Number at \"/ttl\" reads as 0.5, not an integer between -2^53 and 2^53"
        );

        let deep = nest_objects(MAX_DEPTH + 1);
        let refused = canon(&deep).expect_err("Refused");
        assert_eq!(
            refused.to_string(),
            format!(
                "Value at \"{}\" nests past {MAX_DEPTH} containers",
                "/a".repeat(MAX_DEPTH)
            )
        );
    }

    // A pointer carrying a control character has to reach the operator as JSON
    // rather than as Rust Debug syntax
    #[test]
    fn drac_config_canon_quotes_a_pointer_as_json() {
        let value = json!({ "a\u{7}b": 0.5 });
        let refused = canon(&value).expect_err("Refused");
        assert!(
            refused.to_string().contains("\"/a\\u0007b\""),
            "Pointer was not JSON quoted: {refused}"
        );
    }

    #[test]
    fn drac_config_generation_id_debug_prints_hex() {
        // The derived Debug on a byte array prints decimal beside hex filenames
        let digest = id(&json!({})).expect("Hashes");
        let rendered = format!("{digest:?}");
        assert!(
            rendered.contains(&digest.to_string()),
            "Debug lost the hex: {rendered}"
        );
    }

    #[test]
    fn drac_config_canon_refuses_every_float_and_negative_zero() {
        // serde_json reads 56.0 as a float, and v0 refuses it rather than
        // printing an integer a later version would print differently
        for text in ["-0", "-0.0", "1e2", "56.0", "1.5"] {
            let value: Value = serde_json::from_str(text).expect("Parses");
            assert!(
                matches!(canon(&value), Err(CanonError::Number { .. })),
                "Accepted {text}"
            );
        }
    }

    #[test]
    fn drac_config_canon_accepts_2_53_and_refuses_past_it() {
        // Past 2^53 the RFC's number rule prints something other than the
        // literal that was written
        assert!(canon(&json!(9_007_199_254_740_992i64)).is_ok());
        assert!(canon(&json!(-9_007_199_254_740_992i64)).is_ok());
        for refused in [
            9_007_199_254_740_993i64,
            -9_007_199_254_740_993,
            i64::MAX,
            i64::MIN,
        ] {
            assert!(
                matches!(canon(&json!(refused)), Err(CanonError::Number { .. })),
                "Accepted {refused}"
            );
        }
        assert!(matches!(
            canon(&json!(u64::MAX)),
            Err(CanonError::Number { .. })
        ));
    }

    #[test]
    fn drac_config_canon_emits_an_astral_key_before_a_bmp_one() {
        // U+1F602 is the surrogate pair D83D DE02, and D83D is below FB33
        let value = json!({ "\u{fb33}": 1, "\u{1f602}": 2 });
        let got = canon(&value).expect("Canonicalizes");
        let astral = got.find('\u{1f602}').expect("The astral key is present");
        let bmp = got.find('\u{fb33}').expect("The BMP key is present");
        assert!(astral < bmp, "UTF-8 order leaked into member ordering");
    }

    #[test]
    fn drac_config_id_separates_values_and_hashes_the_canonical_bytes() {
        // Pinning one input lets an id that ignores its argument reproduce it
        let a = json!({ "dns": { "port": 53, "catalog": ["ysun.co"] } });
        let b = json!({ "dns": { "port": 5353, "catalog": ["ysun.co"] } });
        assert_ne!(id(&a).expect("Hashes"), id(&b).expect("Hashes"));

        let text = canon(&a).expect("Canonicalizes");
        assert_eq!(
            id(&a).expect("Hashes").to_string(),
            blake3::hash(text.as_bytes()).to_hex().to_string()
        );
    }

    #[test]
    fn drac_config_id_is_the_hex_of_a_blake3_digest() {
        let rendered = id(&json!({})).expect("Canonicalizes").to_string();
        assert_eq!(rendered, blake3::hash(b"{}").to_hex().to_string());
    }

    // Section 3.2.2.2's table, restated from the RFC rather than from the code
    // The domain is 1112064 scalars, which enumerates faster than it proves
    // The hex case reaches the generation id, and reparsing cannot see it
    // because a JSON parser accepts either
    #[test]
    fn drac_config_canon_escapes_exactly_the_rfc_set() {
        for raw in 0u32..=0x10ffff {
            let Some(c) = char::from_u32(raw) else {
                continue;
            };
            let want = match c {
                '\u{8}' => "\\b".to_string(),
                '\u{9}' => "\\t".to_string(),
                '\u{a}' => "\\n".to_string(),
                '\u{c}' => "\\f".to_string(),
                '\u{d}' => "\\r".to_string(),
                '"' => "\\\"".to_string(),
                '\\' => "\\\\".to_string(),
                c if (c as u32) < 0x20 => format!("\\u{:04x}", c as u32),
                c => c.to_string(),
            };
            let mut got = String::new();
            escape(c, &mut got);
            assert_eq!(got, want, "U+{raw:04X} escaped wrongly");
        }
    }

    // The scalars other canonicalizers escape and RFC 8785 leaves alone, driven
    // through canon rather than escape, which is what pins the delimiters
    #[test]
    fn drac_config_canon_round_trips_the_scalars_that_divide_implementations() {
        for c in [
            '/',
            '\u{0}',
            '\u{b}',
            '"',
            '\\',
            '\u{7f}',
            '\u{80}',
            '\u{ad}',
            '\u{2028}',
            '\u{2029}',
            '\u{feff}',
            '\u{1f602}',
        ] {
            let value = Value::String(c.to_string());
            let text = canon(&value).expect("Canonicalizes");
            let mut want = String::new();
            escape(c, &mut want);
            assert_eq!(text, format!("\"{want}\""), "U+{:04X}", c as u32);

            let back: Value = serde_json::from_str(&text).expect("Canonical output reparses");
            assert_eq!(back, value);
        }
    }

    // Nesting only arrays leaves the object arm's bound untested, and an object
    // is the shape a config document actually has
    fn nest(depth: usize, leaf: Value) -> Value {
        let mut value = leaf;
        for level in 0..depth {
            value = if level % 2 == 0 {
                Value::Array(vec![value])
            } else {
                Value::Object([("a".to_string(), value)].into_iter().collect())
            };
        }
        value
    }

    fn nest_objects(depth: usize) -> Value {
        let mut value = Value::Null;
        for _ in 0..depth {
            value = Value::Object([("a".to_string(), value)].into_iter().collect());
        }
        value
    }

    #[test]
    fn drac_config_canon_accepts_exactly_what_it_can_read_back() {
        // Whatever canon accepts, serde_json has to read back
        // One deeper than canon takes is one deeper than serde_json takes
        let deepest = canon(&nest(MAX_DEPTH, Value::Null)).expect("Canonicalizes");
        serde_json::from_str::<Value>(&deepest).expect("The deepest output reads back");

        let past = format!(
            "{}null{}",
            "[".repeat(MAX_DEPTH + 1),
            "]".repeat(MAX_DEPTH + 1)
        );
        assert!(
            serde_json::from_str::<Value>(&past).is_err(),
            "The bound no longer matches what serde_json reads"
        );

        let deepest_objects = canon(&nest_objects(MAX_DEPTH)).expect("Canonicalizes");
        serde_json::from_str::<Value>(&deepest_objects).expect("The deepest object reads back");

        for past in [
            nest(MAX_DEPTH + 1, Value::Null),
            nest(MAX_DEPTH + 1, Value::Array(vec![])),
            nest_objects(MAX_DEPTH + 1),
        ] {
            let Err(CanonError::Depth { pointer, limit }) = canon(&past) else {
                panic!("A value past the bound has to be refused");
            };
            assert_eq!(limit, MAX_DEPTH);
            assert_eq!(
                pointer.matches('/').count(),
                MAX_DEPTH,
                "The pointer lost a level"
            );
        }
    }

    #[test]
    fn drac_config_canon_names_where_a_number_was_refused() {
        let value = json!({ "dns": { "ttl": [1, 0.5] } });
        let Err(CanonError::Number {
            pointer,
            value: shown,
        }) = canon(&value)
        else {
            panic!("A fractional number has to be refused");
        };
        assert_eq!(pointer, "/dns/ttl/1");
        assert_eq!(shown, "0.5");

        // RFC 6901 spells a solidus ~1 and a tilde ~0
        // Decoding takes ~1 first, which is why a tilde has to be escaped too
        let nested = json!({ "a/b": { "c~d": 0.5 } });
        let Err(CanonError::Number { pointer, .. }) = canon(&nested) else {
            panic!("A fractional number has to be refused");
        };
        assert_eq!(pointer, "/a~1b/c~0d");

        // The whole document is the empty pointer, not a phrase
        let Err(CanonError::Number { pointer, .. }) = canon(&json!(0.5)) else {
            panic!("A fractional number has to be refused");
        };
        assert_eq!(pointer, "");
    }

    // Reaching the control range and the astral plane is the point
    // The escape set and the member order both differ from the permissive
    // reading only there
    fn arb_char() -> impl Strategy<Value = char> {
        prop_oneof![
            (0u32..0x20).prop_map(|c| char::from_u32(c).expect("A control character is a scalar")),
            any::<char>(),
            Just('\u{1f602}'),
            Just('\u{fb33}'),
            Just('"'),
            Just('\\'),
            Just('\u{7f}'),
        ]
    }

    fn arb_string() -> impl Strategy<Value = String> {
        proptest::collection::vec(arb_char(), 0..6).prop_map(|cs| cs.into_iter().collect())
    }

    fn arb_value() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            // The range v0 accepts, since a document cannot carry more
            (-MAX_CONTIGUOUS_INT..=MAX_CONTIGUOUS_INT).prop_map(|i| json!(i)),
            arb_string().prop_map(Value::String),
        ];
        leaf.prop_recursive(4, 24, 4, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
                proptest::collection::hash_map(arb_string(), inner, 0..4)
                    .prop_map(|m| Value::Object(m.into_iter().collect())),
            ]
        })
    }

    proptest! {
        // Every clause the output owes, over values the strategy reaches
        // A canon refusing more than it should answers false rather than
        // returning early, which is how a narrowed accept set fails here
        #[test]
        fn drac_config_canon_conforms_over_arbitrary_values(value in arb_value()) {
            prop_assert!(crate::conformance::assert_conforms(&value));
        }

        // An accepted integer survives the double RFC 8785 serializes through
        // That is what keeps a v0 id stable if a later version implements
        // section 3.2.2.3 rather than refusing
        #[test]
        fn drac_config_canon_accepts_an_integer_only_when_its_double_agrees(i in any::<i64>()) {
            match canon(&json!(i)) {
                Ok(text) => {
                    prop_assert_eq!(i as f64 as i64, i);
                    prop_assert_eq!(text, i.to_string());
                }
                Err(CanonError::Number { .. }) => {
                    prop_assert!(i.unsigned_abs() > (1u64 << 53));
                }
                Err(other) => prop_assert!(false, "Refused with {}", other),
            }
        }

        // Section 3.2.3 over arbitrary keys rather than over the one astral
        // example above. The conformance scan reads emitted names, where this
        // starts from the names that went in and finds each one
        #[test]
        fn drac_config_canon_emits_members_in_utf16_order(
            members in proptest::collection::hash_map(
                arb_string(), -MAX_CONTIGUOUS_INT..=MAX_CONTIGUOUS_INT, 0..5)
        ) {
            let value = Value::Object(
                members.iter().map(|(k, v)| (k.clone(), json!(v))).collect());
            let text = canon(&value).expect("Canonicalizes");

            // Every quote inside an encoded name is escaped
            // The opening delimiter is the only raw one
            let mut positions: Vec<(usize, Vec<u16>)> = Vec::new();
            for key in members.keys() {
                let mut encoded = String::new();
                string(key, &mut encoded);
                let found = text
                    .find(&format!("{{{encoded}:"))
                    .or_else(|| text.find(&format!(",{encoded}:")));
                prop_assert!(found.is_some(), "Member {:?} is missing from the output", key);
                positions.push((found.expect("Just checked"), key.encode_utf16().collect()));
            }
            positions.sort_by_key(|(found, _)| *found);

            for pair in positions.windows(2) {
                prop_assert!(pair[0].1 <= pair[1].1, "Members left UTF-16 order");
            }
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Every integer `exact` admits survives the double RFC 8785 serializes
    /// through, over all 2^64 of them.
    /// Sampling reaches that range on roughly one i64 in a thousand, and
    /// reading `exact` rather than restating its range is what makes widening
    /// the accepted set fail here.
    #[kani::proof]
    fn drac_config_every_integer_canon_accepts_survives_a_double() {
        let i: i64 = kani::any();

        // No assumption to narrow, and the covers pin both sides as reachable
        kani::cover!(exact(i));
        kani::cover!(!exact(i));

        assert!(!exact(i) || i as f64 as i64 == i);
    }

    /// Ordering by UTF-16 code unit puts an astral key below an upper BMP one,
    /// where ordering by UTF-8 byte puts it above.
    #[kani::proof]
    #[kani::unwind(5)]
    fn drac_config_utf16_order_puts_an_astral_key_below_the_upper_bmp() {
        let astral: u32 = kani::any();
        let bmp: u16 = kani::any();
        kani::assume((0x10000..=0x10ffff).contains(&astral));
        kani::assume(bmp >= 0xe000);

        kani::cover!();
        kani::cover!(astral == 0x10000);
        kani::cover!(astral == 0x10ffff);
        kani::cover!(bmp == 0xe000);
        kani::cover!(bmp == 0xffff);

        let mut astral_bytes = [0u8; 4];
        let mut bmp_bytes = [0u8; 4];
        let a = char::from_u32(astral)
            .expect("Every astral value is a scalar")
            .encode_utf8(&mut astral_bytes);
        let b = char::from_u32(u32::from(bmp))
            .expect("Every value above the surrogates is a scalar")
            .encode_utf8(&mut bmp_bytes);

        assert!(utf16_order(a, b) == Ordering::Less);
    }
}
