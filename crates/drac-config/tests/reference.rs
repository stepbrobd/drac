// SPDX-FileCopyrightText: 2026 Yifei Sun
// SPDX-License-Identifier: Apache-2.0

//! Vectors from the RFC 8785 reference suite at cyberphone/json-canonicalization.
//!
//! Written as escapes because this repository is ASCII only.
//! To refresh one, read the file of the same name under testdata/input in that
//! repository and re-escape it, rather than retyping the vector by hand.
//! The two named `-accepted` have no upstream file. Each is its upstream twin
//! with the fractional members removed, `1.f.F` and `1.\n` from structures and
//! the whole `numbers` member from values, because v0 refuses a float.

use drac_config::{CanonError, canon};

struct Vector {
    name: &'static str,
    input: &'static str,
    want: &'static str,
}

struct Refusal {
    name: &'static str,
    input: &'static str,
    at: &'static str,
}

const VECTORS: &[Vector] = &[
    Vector {
        name: "arrays",
        input: "[\n  56,\n  {\n    \"d\": true,\n    \"10\": null,\n    \"1\": [ ]\n  }\n]\n",
        want: "[56,{\"1\":[],\"10\":null,\"d\":true}]",
    },
    Vector {
        name: "french",
        input: "{\n  \"peach\": \"This sorting order\",\n  \"p\u{e9}ch\u{e9}\": \"is wrong according to French\",\n  \"p\u{ea}che\": \"but canonicalization MUST\",\n  \"sin\":   \"ignore locale\"\n}\n",
        want: "{\"peach\":\"This sorting order\",\"p\u{e9}ch\u{e9}\":\"is wrong according to French\",\"p\u{ea}che\":\"but canonicalization MUST\",\"sin\":\"ignore locale\"}",
    },
    Vector {
        name: "unicode",
        input: "{\n  \"Unnormalized Unicode\":\"A\\u030a\"\n}\n",
        want: "{\"Unnormalized Unicode\":\"A\u{30a}\"}",
    },
    Vector {
        name: "weird",
        input: "{\n  \"\\u20ac\": \"Euro Sign\",\n  \"\\r\": \"Carriage Return\",\n  \"\\u000a\": \"Newline\",\n  \"1\": \"One\",\n  \"\\u0080\": \"Control\\u007f\",\n  \"\\ud83d\\ude02\": \"Smiley\",\n  \"\\u00f6\": \"Latin Small Letter O With Diaeresis\",\n  \"\\ufb33\": \"Hebrew Letter Dalet With Dagesh\",\n  \"</script>\": \"Browser Challenge\"\n}\n",
        want: "{\"\\n\":\"Newline\",\"\\r\":\"Carriage Return\",\"1\":\"One\",\"</script>\":\"Browser Challenge\",\"\u{80}\":\"Control\u{7f}\",\"\u{f6}\":\"Latin Small Letter O With Diaeresis\",\"\u{20ac}\":\"Euro Sign\",\"\u{1f602}\":\"Smiley\",\"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}",
    },
    Vector {
        name: "values-accepted",
        input: "{\"string\": \"\u{20ac}$\\u000f\\nA'B\\\"\\\\\\\\\\\"/\", \"literals\": [null, true, false]}",
        want: "{\"literals\":[null,true,false],\"string\":\"\u{20ac}$\\u000f\\nA'B\\\"\\\\\\\\\\\"/\"}",
    },
    Vector {
        name: "structures-accepted",
        input: "{\"10\": {}, \"\": \"empty\", \"a\": {}, \"111\": [{\"e\": \"yes\", \"E\": \"no\"}], \"A\": {}}",
        want: "{\"\":\"empty\",\"10\":{},\"111\":[{\"E\":\"no\",\"e\":\"yes\"}],\"A\":{},\"a\":{}}",
    },
];

// The same two vectors whole, which config v0 refuses for a fractional member
// Their other members are kept above rather than discarded
const REFUSALS: &[Refusal] = &[
    Refusal {
        at: "/1/\n",
        name: "structures",
        input: "{\n  \"1\": {\"f\": {\"f\": \"hi\",\"F\": 5} ,\"\\n\": 56.0},\n  \"10\": { },\n  \"\": \"empty\",\n  \"a\": { },\n  \"111\": [ {\"e\": \"yes\",\"E\": \"no\" } ],\n  \"A\": { }\n}",
    },
    Refusal {
        at: "/numbers/0",
        name: "values",
        input: "{\n  \"numbers\": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],\n  \"string\": \"\\u20ac$\\u000F\\u000aA'\\u0042\\u0022\\u005c\\\\\\\"\\/\",\n  \"literals\": [null, true, false]\n}",
    },
];

fn parse(name: &str, input: &str) -> serde_json::Value {
    serde_json::from_str(input).unwrap_or_else(|e| panic!("Vector {name} does not parse: {e}"))
}

#[test]
fn drac_config_canon_matches_the_reference_vectors() {
    for vector in VECTORS {
        let value = parse(vector.name, vector.input);
        let got =
            canon(&value).unwrap_or_else(|e| panic!("Vector {} was refused: {e}", vector.name));
        assert_eq!(got, vector.want, "Vector {} disagreed", vector.name);
    }
}

// Naming the member keeps a refuse-everything encoder from passing
#[test]
fn drac_config_canon_refuses_the_fractional_vectors() {
    for refusal in REFUSALS {
        let value = parse(refusal.name, refusal.input);
        let Err(CanonError::Number { pointer, .. }) = canon(&value) else {
            panic!("Vector {} was not refused", refusal.name);
        };
        assert_eq!(
            pointer, refusal.at,
            "Vector {} refused elsewhere",
            refusal.name
        );
    }
}
