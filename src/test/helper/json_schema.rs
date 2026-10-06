/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! Checks `--mode json` output against `src/tui/json_output.schema.json`, the format's
//! specification, so the two cannot drift apart.
//!
//! A validator for the keywords that schema uses, not a JSON Schema implementation: `type`,
//! `enum`, `const`, `minimum`, `properties`, `required`, `additionalProperties: false`, `items`,
//! `oneOf` and `$ref` into `$defs`. A keyword it does not know fails loudly rather than passing, so
//! the schema cannot grow one this silently ignores. Hand-rolled because the crates that validate
//! JSON Schema pull in a regex engine and an HTTP client for `$ref`s this file never needs.

use anyhow::{Result, bail};
use serde_json::Value;

const SCHEMA: &str = include_str!("../../tui/json_output.schema.json");

/// Keywords that document rather than constrain.
const ANNOTATIONS: [&str; 5] = ["$schema", "$id", "$defs", "title", "description"];
/// Keywords [`violation`] checks.
const CHECKED: [&str; 10] = [
    "$ref",
    "oneOf",
    "type",
    "enum",
    "const",
    "minimum",
    "properties",
    "required",
    "additionalProperties",
    "items",
];

/// Fails, saying where, unless `json` is what the schema allows.
pub fn check_json_output(json: &str) -> Result<()> {
    let schema: Value = serde_json::from_str(SCHEMA)?;
    let value: Value = serde_json::from_str(json)?;
    if let Some(broken) = violation(&schema, &schema, &value, "$") {
        bail!("the JSON output does not match json_output.schema.json: {broken}\n{json}");
    }
    Ok(())
}

/// Where `value` breaks `schema`, or `None`; `root` holds the `$defs` a `$ref` names.
fn violation(root: &Value, schema: &Value, value: &Value, at: &str) -> Option<String> {
    for keyword in schema
        .as_object()
        .into_iter()
        .flat_map(|object| object.keys())
    {
        assert!(
            ANNOTATIONS.contains(&keyword.as_str()) || CHECKED.contains(&keyword.as_str()),
            "json_output.schema.json uses {keyword}, which this validator does not check"
        );
    }
    if let Some(reference) = schema["$ref"].as_str() {
        let name = reference.strip_prefix("#/$defs/").expect("a local $ref");
        assert!(root["$defs"].get(name).is_some(), "no $defs/{name}");
        return violation(root, &root["$defs"][name], value, at);
    }
    if let Some(options) = schema["oneOf"].as_array() {
        let failures: Vec<String> = options
            .iter()
            .filter_map(|option| violation(root, option, value, at))
            .collect();
        return match options.len() - failures.len() {
            1 => None,
            0 => Some(format!("{at} matches no option: {failures:?}")),
            n => Some(format!("{at} matches {n} options")),
        };
    }
    if let Some(types) = schema.get("type") {
        let types: Vec<&str> = match types {
            Value::Array(types) => types.iter().filter_map(Value::as_str).collect(),
            other => vec![other.as_str().expect("a type name")],
        };
        let is = |name: &str| match name {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "integer" => value.is_u64() || value.is_i64(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            other => panic!("type {other} is not checked"),
        };
        if !types.iter().any(|name| is(name)) {
            return Some(format!("{at}: {value} is not {types:?}"));
        }
    }
    if let Some(allowed) = schema["enum"].as_array()
        && !allowed.contains(value)
    {
        return Some(format!("{at}: {value} is not one of {allowed:?}"));
    }
    if let Some(constant) = schema.get("const")
        && constant != value
    {
        return Some(format!("{at}: {value} is not {constant}"));
    }
    if let (Some(minimum), Some(number)) = (schema["minimum"].as_u64(), value.as_u64())
        && number < minimum
    {
        return Some(format!("{at}: {number} is under {minimum}"));
    }
    if let Some(object) = value.as_object() {
        for name in schema["required"].as_array().into_iter().flatten() {
            let name = name.as_str().expect("a field name");
            if !object.contains_key(name) {
                return Some(format!("{at}.{name} is missing"));
            }
        }
        for (name, field) in object {
            let at = format!("{at}.{name}");
            match schema["properties"].get(name) {
                Some(field_schema) => {
                    if let Some(broken) = violation(root, field_schema, field, &at) {
                        return Some(broken);
                    }
                }
                None if schema["additionalProperties"] == Value::Bool(false) => {
                    return Some(format!("{at} is not in the schema"));
                }
                None => {}
            }
        }
    }
    if let (Some(items), Some(array)) = (schema.get("items"), value.as_array()) {
        for (index, item) in array.iter().enumerate() {
            if let Some(broken) = violation(root, items, item, &format!("{at}[{index}]")) {
                return Some(broken);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_diff() -> Value {
        serde_json::json!({
            "before": { "path": "a.rs", "language": "Rust", "hunks": [
                { "operation": "delete",
                  "range": { "start_row": 1, "start_column": 4, "end_row": 2, "end_column": 0 } }
            ] },
            "after": { "path": "b.rs", "language": null, "hunks": [] },
            "large_residual": false
        })
    }

    #[test]
    fn the_schema_allows_a_text_diff() {
        check_json_output(&text_diff().to_string()).unwrap();
    }

    #[test]
    fn the_schema_refuses_a_field_it_does_not_name() {
        let mut diff = text_diff();
        diff["before"]["hunks"][0]["colour"] = "red".into();
        let error = check_json_output(&diff.to_string())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("$.before.hunks[0].colour is not in the schema"),
            "{error}"
        );
    }

    #[test]
    fn the_schema_refuses_a_missing_field_a_wrong_value_and_a_wrong_type() {
        for (change, expected) in [
            (
                ("large_residual", Value::Null),
                "$.large_residual: null is not [\"boolean\"]",
            ),
            (
                ("summary", "renamed".into()),
                "$.summary: \"renamed\" is not one of",
            ),
            (("binary", false.into()), "$.binary: false is not true"),
        ] {
            let mut diff = text_diff();
            diff[change.0] = change.1;
            let error = check_json_output(&diff.to_string())
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{error}");
        }
        let mut diff = text_diff();
        diff["after"].as_object_mut().unwrap().remove("hunks");
        let error = check_json_output(&diff.to_string())
            .unwrap_err()
            .to_string();
        assert!(error.contains("$.after.hunks is missing"), "{error}");
    }

    #[test]
    fn a_tagged_value_must_match_exactly_one_kind() {
        let mut diff = text_diff();
        diff["binary"] = true.into();
        diff["content"] = serde_json::json!({ "kind": "sound" });
        let error = check_json_output(&diff.to_string())
            .unwrap_err()
            .to_string();
        assert!(error.contains("$.content matches no option"), "{error}");
    }
}
