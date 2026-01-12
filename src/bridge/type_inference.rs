use serde_json::Value;
use crate::bridge::protocol::{TnkValue, JsonPrimitive, DictItem};

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
                TnkValue::Value { value: Some(JsonPrimitive::Float(0.0)) }
            }
        },
        Value::String(s) => TnkValue::Value { value: Some(JsonPrimitive::String(s)) },
        Value::Array(arr) => TnkValue::List { 
            items: arr.into_iter().map(from_value).collect() 
        },
        Value::Object(map) => {
            // V1.7.0: 能動的解釈 (Active Interpretation)
            // まず Tagged Union (TnkValue の定義) に合致するかを試行する
            let json_obj = Value::Object(map.clone());
            if let Ok(tnk) = serde_json::from_value::<TnkValue>(json_obj) {
                // kind フィールドが存在し、正しい構造を持っていればそれを採用
                // (タグ付き Enum なので serde_json::from_value が最適)
                return tnk;
            }

            // 合致しない場合は一般的な Dictionary として解釈
            let items = map.into_iter().map(|(k, v)| DictItem {
                key: TnkValue::Value { value: Some(JsonPrimitive::String(k)) },
                value: from_value(v)
            }).collect();
            TnkValue::Dict { items }
        }
    }
}
