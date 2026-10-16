//! Unit 類型檔的 JSON Schema 產生器（規格 §4.2、§8.2）。
//!
//! 唯一事實來源仍是 [`crate::naming`] 的前綴表；此處透過 [`Prefix::ALL`]
//! 遍歷產生 `standards/schema/unit-type.schema.json`，與程式碼版 SC2／SC3 一致。

use crate::naming::Prefix;
use serde_json::{Map, Value};

const SCALARS: &[&str] = &[
    "bool", "int16", "uint16", "int32", "uint32", "int64", "uint64", "float32", "float64",
];

const STRING_N: &str = "^string:[1-9][0-9]*$";
const ARRAY: &str = "^(bool|int16|uint16|int32|uint32|int64|uint64|float32|float64)\\[([1-9]|[1-9][0-9]|[1-4][0-9][0-9]|500)\\]$";

const CTRL_PREFIX_PATTERN: &str = "^(cmd|man|ena|par|at|sts|error|count|pv|msg)_";

fn scalar_enum() -> Value {
    Value::Array(
        SCALARS
            .iter()
            .map(|s| Value::String((*s).to_owned()))
            .collect(),
    )
}

fn type_any() -> Value {
    serde_json::json!({
        "anyOf": [
            { "enum": scalar_enum() },
            { "type": "string", "pattern": STRING_N },
            { "type": "string", "pattern": ARRAY },
        ]
    })
}

fn string_n_only() -> Value {
    serde_json::json!({ "type": "string", "pattern": STRING_N })
}

/// 每個前綴一條 schema，呼叫者以 `Prefix::ALL` 遍歷組裝。
/// 接腳只有 `type`、`label`、`unit`（與 `par_` 的 min／max／default、`error_` 的 message）；
/// 其餘屬性由 `additionalProperties: false` 擋下。
fn per_prefix(prefix: Prefix) -> Value {
    let label = serde_json::json!({ "type": "string" });
    let unit = serde_json::json!({ "type": "string" });
    match prefix {
        Prefix::Cmd | Prefix::Man | Prefix::Ena | Prefix::At | Prefix::Sts | Prefix::Count => {
            serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "type": type_any(),
                    "label": label,
                    "unit": unit,
                },
            })
        }
        Prefix::Par => serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["type", "min", "max", "default"],
            "properties": {
                "type": type_any(),
                "min": { "type": "number" },
                "max": { "type": "number" },
                "default": { "type": "number" },
                "unit": unit,
                "label": label,
            },
        }),
        Prefix::Error => serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["message"],
            "properties": {
                "type": { "enum": ["bool"] },
                "message": { "type": "string", "minLength": 1 },
                "label": label,
                "unit": unit,
            },
        }),
        Prefix::Pv => serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["type"],
            "properties": {
                "type": type_any(),
                "label": label,
                "unit": unit,
            },
        }),
        Prefix::Msg => serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["type"],
            "properties": {
                "type": string_n_only(),
                "label": label,
                "unit": unit,
            },
        }),
    }
}

fn field_pin() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["type"],
        "properties": {
            "type": type_any(),
            "required": { "type": "boolean" },
            "label": { "type": "string" },
            "unit": { "type": "string" },
        },
    })
}

fn field_map() -> Value {
    serde_json::json!({
        "type": "object",
        "propertyNames": { "not": { "pattern": CTRL_PREFIX_PATTERN } },
        "additionalProperties": field_pin(),
    })
}

/// 產生 `unit-type.schema.json` 的內容（pretty JSON）。
pub fn generate_unit_type_schema() -> String {
    let mut patterns = Map::new();
    for prefix in Prefix::ALL {
        patterns.insert(format!("^{}_", prefix.info().name), per_prefix(prefix));
    }
    let schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Unit 類型檔",
        "type": "object",
        "additionalProperties": false,
        "required": ["id", "version", "title"],
        "properties": {
            "id": { "type": "string", "pattern": "^[a-z][a-z0-9_]*$" },
            "version": { "type": "integer", "minimum": 1 },
            "title": { "type": "string", "minLength": 1 },
            "field": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "in": field_map(),
                    "out": field_map(),
                },
            },
            "ctrl": {
                "type": "object",
                "patternProperties": Value::Object(patterns),
                "unevaluatedProperties": false,
            },
            "parts": {
                "type": "object",
                "propertyNames": { "pattern": "^[A-Za-z0-9_]+$" },
                "additionalProperties": {
                    "type": "string",
                    "not": { "anyOf": [
                        { "enum": scalar_enum() },
                        { "type": "string", "pattern": STRING_N },
                        { "type": "string", "pattern": ARRAY },
                    ] },
                },
            },
        },
    });
    serde_json::to_string_pretty(&schema).expect("schema 必須可序列化") + "\n"
}
