use serde_json::{Map, Value as JsonValue};

use crate::Value;

#[derive(Debug, thiserror::Error)]
pub enum JsonBridgeError {
    #[error("invalid json argument list: {0}")]
    InvalidArgumentList(String),
    #[error("only i64 numbers are supported")]
    UnsupportedNumber,
    #[error("tagged variant objects must include a string `tag` field")]
    MissingVariantTag,
    #[error("tagged variant objects must use an array `fields` value")]
    InvalidVariantFields,
}

pub fn values_from_json_str(input: &str) -> Result<Vec<Value>, JsonBridgeError> {
    let values: Vec<JsonValue> = serde_json::from_str(input)
        .map_err(|error| JsonBridgeError::InvalidArgumentList(error.to_string()))?;
    values.into_iter().map(value_from_json).collect()
}

pub fn value_from_json(value: JsonValue) -> Result<Value, JsonBridgeError> {
    match value {
        JsonValue::Bool(flag) => Ok(Value::Bool(flag)),
        JsonValue::Number(number) => number
            .as_i64()
            .map(Value::Int)
            .ok_or(JsonBridgeError::UnsupportedNumber),
        JsonValue::String(text) => Ok(Value::String(text)),
        JsonValue::Array(items) => Ok(Value::List(
            items
                .into_iter()
                .map(value_from_json)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        JsonValue::Null => Ok(Value::Unit),
        JsonValue::Object(map) => parse_variant_object(map),
    }
}

pub fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Unit => JsonValue::Null,
        Value::Int(number) => serde_json::json!(number),
        Value::Bool(flag) => serde_json::json!(flag),
        Value::String(text) => serde_json::json!(text),
        Value::List(items) | Value::Tuple(items) => {
            JsonValue::Array(items.iter().map(value_to_json).collect::<Vec<_>>())
        }
        Value::Variant { name, fields } => serde_json::json!({
            "tag": name,
            "fields": fields.iter().map(value_to_json).collect::<Vec<_>>(),
        }),
        Value::Function(name) => serde_json::json!({
            "tag": "Function",
            "fields": [name],
        }),
        Value::Closure { .. } => serde_json::json!({
            "tag": "Closure",
            "fields": [],
        }),
        Value::IterUnfold { .. } => serde_json::json!({
            "tag": "Iter",
            "fields": [],
        }),
        Value::Error => serde_json::json!({
            "tag": "Error",
            "fields": [],
        }),
    }
}

fn parse_variant_object(map: Map<String, JsonValue>) -> Result<Value, JsonBridgeError> {
    let tag = map
        .get("tag")
        .and_then(JsonValue::as_str)
        .ok_or(JsonBridgeError::MissingVariantTag)?
        .to_owned();
    let fields = match map.get("fields") {
        Some(JsonValue::Array(items)) => items.clone(),
        Some(_) => return Err(JsonBridgeError::InvalidVariantFields),
        None => Vec::new(),
    };
    Ok(Value::Variant {
        name: tag,
        fields: fields
            .into_iter()
            .map(value_from_json)
            .collect::<Result<Vec<_>, _>>()?,
    })
}
