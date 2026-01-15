use crate::bridge::protocol::{DictItem, JsonPrimitive, TnkValue};
use serde_json::Value;

pub fn from_value(v: Value) -> TnkValue {
    match v {
        Value::Null => TnkValue::Value { value: None },
        Value::Bool(b) => TnkValue::Value {
            value: Some(JsonPrimitive::Bool(b)),
        },
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TnkValue::Value {
                    value: Some(JsonPrimitive::Int(i)),
                }
            } else if let Some(f) = n.as_f64() {
                TnkValue::Value {
                    value: Some(JsonPrimitive::Float(f)),
                }
            } else {
                TnkValue::Value {
                    value: Some(JsonPrimitive::Float(0.0)),
                }
            }
        }
        Value::String(s) => TnkValue::Value {
            value: Some(JsonPrimitive::String(s)),
        },
        Value::Array(arr) => TnkValue::List {
            items: arr.into_iter().map(from_value).collect(),
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
            let items = map
                .into_iter()
                .map(|(k, v)| DictItem {
                    key: TnkValue::Value {
                        value: Some(JsonPrimitive::String(k)),
                    },
                    value: from_value(v),
                })
                .collect();
            TnkValue::Dict { items }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_from_value_primitives() {
        assert_eq!(from_value(Value::Null), TnkValue::Value { value: None });
        assert_eq!(
            from_value(Value::Bool(true)),
            TnkValue::Value {
                value: Some(JsonPrimitive::Bool(true))
            }
        );
        assert_eq!(
            from_value(Value::Number(serde_json::Number::from(7))),
            TnkValue::Value {
                value: Some(JsonPrimitive::Int(7))
            }
        );
        assert_eq!(
            from_value(Value::Number(serde_json::Number::from_f64(2.5).unwrap())),
            TnkValue::Value {
                value: Some(JsonPrimitive::Float(2.5))
            }
        );
        assert_eq!(
            from_value(Value::String("hi".to_string())),
            TnkValue::Value {
                value: Some(JsonPrimitive::String("hi".to_string()))
            }
        );
    }

    #[test]
    fn test_from_value_array_and_object_as_dict() {
        let v = json!([1, "a"]);
        assert_eq!(
            from_value(v),
            TnkValue::List {
                items: vec![
                    TnkValue::Value {
                        value: Some(JsonPrimitive::Int(1))
                    },
                    TnkValue::Value {
                        value: Some(JsonPrimitive::String("a".to_string()))
                    },
                ]
            }
        );

        let obj = json!({"k": 1});
        let dict = from_value(obj);
        match dict {
            TnkValue::Dict { items } => {
                assert_eq!(items.len(), 1);
                assert_eq!(
                    items[0].key,
                    TnkValue::Value {
                        value: Some(JsonPrimitive::String("k".to_string()))
                    }
                );
                assert_eq!(
                    items[0].value,
                    TnkValue::Value {
                        value: Some(JsonPrimitive::Int(1))
                    }
                );
            }
            _ => panic!("Expected Dict"),
        }
    }

    #[test]
    fn test_from_value_prefers_tagged_tnkvalue() {
        let tagged = json!({
            "kind": "handle",
            "id": "h1",
            "type": "str",
            "repr": "'x'",
            "str": "x",
            "session_id": "s1"
        });
        let value = from_value(tagged);
        match value {
            TnkValue::Handle {
                id,
                type_,
                str_,
                session_id,
                ..
            } => {
                assert_eq!(id, "h1");
                assert_eq!(type_, "str");
                assert_eq!(str_, "x");
                assert_eq!(session_id, "s1");
            }
            _ => panic!("Expected Handle"),
        }
    }
}
