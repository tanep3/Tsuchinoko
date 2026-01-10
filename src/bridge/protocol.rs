use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Data Types (Tagged Union) ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TnkValue {
    Value {
        value: Option<JsonPrimitive>,
    },
    Handle {
        id: String,
        #[serde(rename = "type")]
        type_: String,
        repr: String,
        session_id: String,
    },
    List {
        items: Vec<TnkValue>,
    },
    Tuple {
        items: Vec<TnkValue>,
    },
    Dict {
        items: Vec<DictItem>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonPrimitive {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictItem {
    pub key: TnkValue,
    pub value: TnkValue,
}


// --- From Implementations ---

impl From<i64> for TnkValue {
    fn from(n: i64) -> Self {
        TnkValue::Value {
            value: Some(JsonPrimitive::Int(n)),
        }
    }
}

impl From<i32> for TnkValue {
    fn from(n: i32) -> Self {
        TnkValue::Value {
            value: Some(JsonPrimitive::Int(n as i64)),
        }
    }
}

impl From<f64> for TnkValue {
    fn from(n: f64) -> Self {
        TnkValue::Value {
            value: Some(JsonPrimitive::Float(n)),
        }
    }
}

impl From<f32> for TnkValue {
    fn from(n: f32) -> Self {
        TnkValue::Value {
            value: Some(JsonPrimitive::Float(n as f64)),
        }
    }
}

impl From<bool> for TnkValue {
    fn from(b: bool) -> Self {
        TnkValue::Value {
            value: Some(JsonPrimitive::Bool(b)),
        }
    }
}

impl From<String> for TnkValue {
    fn from(s: String) -> Self {
        TnkValue::Value {
            value: Some(JsonPrimitive::String(s)),
        }
    }
}

impl From<&str> for TnkValue {
    fn from(s: &str) -> Self {
        TnkValue::Value {
            value: Some(JsonPrimitive::String(s.to_string())),
        }
    }
}

impl From<Value> for TnkValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => TnkValue::Value { value: None },
            Value::Bool(b) => TnkValue::from(b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    TnkValue::from(i)
                } else if let Some(f) = n.as_f64() {
                    TnkValue::from(f)
                } else {
                    TnkValue::Value {
                        value: Some(JsonPrimitive::Float(0.0)),
                    } // fallback
                }
            }
            Value::String(s) => TnkValue::from(s),
            Value::Array(arr) => {
                let items = arr.into_iter().map(TnkValue::from).collect();
                TnkValue::List { items }
            }
            Value::Object(obj) => {
                let items = obj
                    .into_iter()
                    .map(|(k, v)| DictItem {
                        key: TnkValue::from(k),
                        value: TnkValue::from(v),
                    })
                    .collect();
                TnkValue::Dict { items }
            }
        }
    }
}


// --- Helper Methods ---

impl TnkValue {
    pub fn is_none(&self) -> bool {
        matches!(self, TnkValue::Value { value: None })
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            TnkValue::Value { value: Some(JsonPrimitive::Int(n)) } => Some(*n),
            TnkValue::Value { value: Some(JsonPrimitive::Float(f)) } => Some(*f as i64),
            _ => None,
        }
    }
    
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            TnkValue::Value { value: Some(JsonPrimitive::Float(f)) } => Some(*f),
            TnkValue::Value { value: Some(JsonPrimitive::Int(n)) } => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            TnkValue::Value {
                value: Some(JsonPrimitive::Bool(b)),
            } => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            TnkValue::Value {
                value: Some(JsonPrimitive::String(s)),
            } => Some(s.as_str()),
            _ => None,
        }
    }
}

impl From<TnkValue> for serde_json::Value {
    fn from(val: TnkValue) -> Self {
        serde_json::to_value(&val).unwrap_or(serde_json::Value::Null)
    }
}

// From<Value> for TnkValue is already implemented above (line 87).

impl std::fmt::Display for JsonPrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonPrimitive::Bool(b) => write!(f, "{}", b),
            JsonPrimitive::Int(n) => write!(f, "{}", n),
            JsonPrimitive::Float(n) => write!(f, "{}", n),
            JsonPrimitive::String(s) => write!(f, "{:?}", s), // Use debug for string to include quotes
        }
    }
}

impl std::fmt::Display for TnkValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TnkValue::Value { value: Some(primitive) } => write!(f, "{}", primitive),
            TnkValue::Value { value: None } => write!(f, "null"),
            TnkValue::Handle { id, repr, .. } => write!(f, "<Handle:{}> ({})", id, repr),
            TnkValue::List { items } => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            TnkValue::Tuple { items } => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                // Add trailing comma for single-element tuples to distinguish from parenthesized expressions
                if items.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            TnkValue::Dict { items } => {
                write!(f, "{{")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", item.key, item.value)?;
                }
                write!(f, "}}")
            }
        }
    }
}

// --- Commands ---

#[derive(Debug, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    CallFunction {
        session_id: String,
        req_id: Option<String>,
        target: String,
        args: Vec<TnkValue>,
    },
    CallMethod {
        session_id: String,
        req_id: Option<String>,
        target: String,
        method: String,
        args: Vec<TnkValue>,
    },
    GetAttribute {
        session_id: String,
        req_id: Option<String>,
        target: String,
        name: String,
    },
    GetItem {
        session_id: String,
        req_id: Option<String>,
        target: String,
        key: TnkValue,
    },
    Slice {
        session_id: String,
        req_id: Option<String>,
        target: String,
        start: TnkValue,
        stop: TnkValue,
        step: TnkValue,
    },
    Iter {
        session_id: String,
        req_id: Option<String>,
        target: String,
    },
    IterNextBatch {
        session_id: String,
        req_id: Option<String>,
        target: String,
        batch_size: usize,
    },
    Delete {
        session_id: String,
        req_id: Option<String>,
        target: String,
    },
}

// --- Response ---

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Response {
    Ok {
        req_id: Option<String>,
        value: TnkValue,
        meta: Option<ResponseMeta>,
    },
    Error {
        req_id: Option<String>,
        error: BridgeErrorDetail,
    },
}

#[derive(Debug, Deserialize)]
pub struct ResponseMeta {
    pub done: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BridgeErrorDetail {
    pub code: String,
    pub py_type: Option<String>,
    pub message: String,
    pub traceback: Option<String>,
    pub op: Option<Value>,
}

// Comparison helpers for generated code
impl PartialEq<f64> for TnkValue {
    fn eq(&self, other: &f64) -> bool {
        match self {
            TnkValue::Value { value: Some(JsonPrimitive::Float(n)) } => (n - other).abs() < f64::EPSILON,
            TnkValue::Value { value: Some(JsonPrimitive::Int(n)) } => (*n as f64 - other).abs() < f64::EPSILON,
            _ => false,
        }
    }
}

impl PartialEq<i64> for TnkValue {
    fn eq(&self, other: &i64) -> bool {
        match self {
            TnkValue::Value { value: Some(JsonPrimitive::Int(n)) } => *n == *other,
            TnkValue::Value { value: Some(JsonPrimitive::Float(n)) } => (*n as i64) == *other, // Rough comparison
            _ => false,
        }
    }
}

impl PartialEq<bool> for TnkValue {
    fn eq(&self, other: &bool) -> bool {
        match self {
            TnkValue::Value { value: Some(JsonPrimitive::Bool(b)) } => b == other,
            _ => false,
        }
    }
}

impl PartialEq<&str> for TnkValue {
    fn eq(&self, other: &&str) -> bool {
        match self {
            TnkValue::Value { value: Some(JsonPrimitive::String(s)) } => s == *other,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tnk_value_serialization() {
        let v = TnkValue::Value { value: Some(JsonPrimitive::Float(42.0)) };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"kind":"value","value":42.0}"#);

        let h = TnkValue::Handle {
            id: "h1".to_string(),
            type_: "str".to_string(),
            repr: "'foo'".to_string(),
            session_id: "s1".to_string(),
        };
        let h_json = serde_json::to_string(&h).unwrap();
        assert!(h_json.contains(r#""kind":"handle""#));
        assert!(h_json.contains(r#""type":"str""#));
    }

    #[test]
    fn test_command_serialization() {
        let cmd = Command::CallMethod {
            session_id: "sess".into(),
            req_id: Some("req".into()),
            target: "h1".into(),
            method: "foo".into(),
            args: vec![],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains(r#""cmd":"call_method""#));
        assert!(json.contains(r#""req_id":"req""#));
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"
            {
                "kind": "ok",
                "req_id": "r1",
                "value": {"kind": "value", "value": "test"},
                "meta": {"done": true}
            }
        "#;
        let resp: Response = serde_json::from_str(json).unwrap();
        match resp {
            Response::Ok { req_id, value, meta } => {
                assert_eq!(req_id.unwrap(), "r1");
                match value {
                    TnkValue::Value { value: Some(JsonPrimitive::String(s)) } => assert_eq!(s, "test"),
                    _ => panic!("Wrong value"),
                }
                assert_eq!(meta.unwrap().done, Some(true));
            },
            _ => panic!("Expected Ok"),
        }
    }
}
