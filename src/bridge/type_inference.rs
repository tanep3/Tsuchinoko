use serde_json::Value;
use crate::bridge::protocol::{TnkValue, JsonPrimitive, DictItem};

/// Infer TnkValue from generic serde_json::Value
/// This function handles the strict type conversion required for Python bridge
pub fn from_value(v: Value) -> TnkValue {
    match v {
        Value::Null => TnkValue::Value { value: None },
        Value::Bool(b) => TnkValue::Value { value: Some(JsonPrimitive::Bool(b)) },
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TnkValue::Value { value: Some(JsonPrimitive::Int(i)) }
            } else if let Some(f) = n.as_f64() {
                TnkValue::Value { value: Some(JsonPrimitive::Float(f)) }
            } else {
                // Should not happen for valid JSON numbers
                TnkValue::Value { value: Some(JsonPrimitive::Float(0.0)) }
            }
        },
        Value::String(s) => TnkValue::Value { value: Some(JsonPrimitive::String(s)) },
        Value::Array(arr) => TnkValue::List { 
            items: arr.into_iter().map(from_value).collect() 
        },
        Value::Object(map) => {
            // Treat all objects as Dictionaries
            // The handling of specific handles ({ "__handle__": ... }) should typically happen at a higher layer
            // or we can strictly convert them here if we want automatic wrapping.
            // For now, consistent Dict conversion.
            let items = map.into_iter().map(|(k, v)| DictItem {
                key: TnkValue::Value { value: Some(JsonPrimitive::String(k)) },
                value: from_value(v)
            }).collect();
            TnkValue::Dict { items }
        }
    }
}
