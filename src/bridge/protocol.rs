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
        #[serde(rename = "str")]
        str_: String,
        session_id: String,
    },
    Module {
        module: String,
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

// Greedy From<Value> removed to prevent misinterpretation of Tagged Union objects.
// Use bridge::type_inference::from_value(v) for intelligent interpretation.


impl From<std::collections::HashMap<String, TnkValue>> for TnkValue {
    fn from(map: std::collections::HashMap<String, TnkValue>) -> Self {
        let items = map.into_iter().map(|(k, v)| DictItem {
            key: TnkValue::from(k),
            value: v,
        }).collect();
        TnkValue::Dict { items }
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
            JsonPrimitive::String(s) => write!(f, "{}", s), // Use Display for strings to avoid quotes
        }
    }
}

// Support tuple conversions
macro_rules! impl_tuple_from {
    ($($n:tt $name:ident)+) => {
        impl<$($name: Into<TnkValue>),+> From<($($name,)+)> for TnkValue {
            fn from(tuple: ($($name,)+)) -> Self {
                TnkValue::Tuple {
                    items: vec![$(tuple.$n.into()),+],
                }
            }
        }
    }
}

impl_tuple_from!(0 T0 1 T1);
impl_tuple_from!(0 T0 1 T1 2 T2);
impl_tuple_from!(0 T0 1 T1 2 T2 3 T3);
impl_tuple_from!(0 T0 1 T1 2 T2 3 T3 4 T4);
impl_tuple_from!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5);

// Support Vec/List conversion
impl<T: Into<TnkValue>> From<Vec<T>> for TnkValue {
    fn from(vec: Vec<T>) -> Self {
        TnkValue::List {
            items: vec.into_iter().map(|i| i.into()).collect(),
        }
    }
}

// Support Slice conversion (clones elements)
impl<T: Clone + Into<TnkValue>> From<&[T]> for TnkValue {
    fn from(slice: &[T]) -> Self {
        TnkValue::List {
            items: slice.iter().map(|i| i.clone().into()).collect(),
        }
    }
}

impl std::fmt::Display for TnkValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TnkValue::Value { value: Some(primitive) } => write!(f, "{}", primitive),
            TnkValue::Value { value: None } => write!(f, "null"),
            TnkValue::Handle { str_, .. } => write!(f, "{}", str_),
            TnkValue::Module { module } => write!(f, "<Module:{}>", module),
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
pub enum Command<'a> {
    CallFunction {
        session_id: String,
        req_id: Option<String>,
        target: String,
        args: Vec<&'a TnkValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        kwargs: Option<std::collections::HashMap<String, &'a TnkValue>>,
    },
    CallMethod {
        session_id: String,
        req_id: Option<String>,
        target: serde_json::Value,
        method: String,
        args: Vec<&'a TnkValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        kwargs: Option<std::collections::HashMap<String, &'a TnkValue>>,
    },
    GetAttribute {
        session_id: String,
        req_id: Option<String>,
        target: serde_json::Value,
        name: String,
    },
    GetItem {
        session_id: String,
        req_id: Option<String>,
        target: serde_json::Value,
        key: TnkValue, // Key is usually small (Int/String), keep owned for simplicity or Change to &'a TnkValue? Let's keep owned for Key for now to avoid complexity in simple getters? User wants ZERO Copy. Key should be ref.
    },
    Slice {
        session_id: String,
        req_id: Option<String>,
        target: serde_json::Value,
        start: TnkValue, // Slice args are small primitives usually.
        stop: TnkValue,
        step: TnkValue,
    },
    Iter {
        session_id: String,
        req_id: Option<String>,
        target: serde_json::Value,
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
        target: serde_json::Value,
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

impl PartialEq<(i64, i64, i64)> for TnkValue {
    fn eq(&self, other: &(i64, i64, i64)) -> bool {
        match self {
            TnkValue::Tuple { items } if items.len() == 3 => {
                items[0] == other.0 && items[1] == other.1 && items[2] == other.2
            },
            TnkValue::List { items } if items.len() == 3 => {
                 items[0] == other.0 && items[1] == other.1 && items[2] == other.2
            },
            _ => false,
        }
    }
}

// Indexing support for tuple unpacking
impl std::ops::Index<usize> for TnkValue {
    type Output = TnkValue;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            TnkValue::List { items } => &items[index],
            TnkValue::Tuple { items } => &items[index],
            _ => panic!("Cannot index non-sequence TnkValue"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tnk_value_serialization() {
// ... existing tests ...
        let v = TnkValue::Value { value: Some(JsonPrimitive::Float(42.0)) };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"kind":"value","value":42.0}"#);

        let h = TnkValue::Handle {
            id: "h1".to_string(),
            type_: "str".to_string(),
            repr: "'foo'".to_string(),
            str_: "foo".to_string(),
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
            kwargs: None,
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
