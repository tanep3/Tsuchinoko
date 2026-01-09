//! Type definitions

/// Rust types
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,                        // i64
    Float,                      // f64
    String,                     // String
    Bool,                       // bool
    List(Box<Type>),            // Vec<T>
    Set(Box<Type>),             // HashSet<T> (V1.5.0)
    Tuple(Vec<Type>),           // (T, U, ...)
    Dict(Box<Type>, Box<Type>), // HashMap<K, V>
    Optional(Box<Type>),        // Option<T>
    Ref(Box<Type>),             // &T
    MutRef(Box<Type>),          // &mut T
    Func {
        params: Vec<Type>,
        ret: Box<Type>,
        is_boxed: bool,
        /// V1.5.2: Whether this function may raise (Result で wrap される)
        may_raise: bool,
    },
    Unit,           // ()
    Struct(String), // User-defined struct
    Any,            // Dynamic Type (maps to serde_json::Value)
    Unknown,        // Not yet inferred
}

impl Type {
    /// Convert Python type hint to Rust type
    pub fn from_python_hint(name: &str, params: &[Type]) -> Self {
        match name {
            "Any" | "any" | "object" => Type::Any,
            "int" => Type::Int,
            "float" => Type::Float,
            "str" => Type::String,
            "bool" => Type::Bool,
            "list" | "List" => {
                let inner = params.first().cloned().unwrap_or(Type::Unknown);
                Type::List(Box::new(inner))
            }
            "tuple" | "Tuple" => {
                if params.is_empty() {
                    Type::List(Box::new(Type::Unknown))
                } else {
                    Type::Tuple(params.to_vec())
                }
            }
            "dict" | "Dict" => {
                let key = params.first().cloned().unwrap_or(Type::Unknown);
                let val = params.get(1).cloned().unwrap_or(Type::Unknown);
                Type::Dict(Box::new(key), Box::new(val))
            }
            "set" | "Set" => {
                let inner = params.first().cloned().unwrap_or(Type::Unknown);
                Type::Set(Box::new(inner))
            }
            "Optional" => {
                let inner = params.first().cloned().unwrap_or(Type::Unknown);
                Type::Optional(Box::new(inner))
            }
            // Internal: [int, int] parsed as param list for Callable
            "__param_list__" => Type::Tuple(params.to_vec()),
            "None" => Type::Unit,
            // Callable[[Param1, Param2], ReturnType] -> fn(Param1, Param2) -> ReturnType
            "Callable" => {
                // params[0] should be a Tuple-like list of param types
                // params[1] should be the return type
                let param_types = if let Some(Type::List(inner)) = params.first() {
                    // If it's a list, extract the inner type (simplified - single type)
                    vec![*inner.clone()]
                } else if let Some(Type::Tuple(types)) = params.first() {
                    types.clone()
                } else {
                    // Default: no params
                    vec![]
                };
                let ret = params.get(1).cloned().unwrap_or(Type::Unknown);
                Type::Func {
                    params: param_types,
                    ret: Box::new(ret),
                    is_boxed: true, // Callable implies generic/boxed function object
                    may_raise: false,
                }
            }
            // Check if it's a user-defined type (capitalized name)
            name if name
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false) =>
            {
                Type::Struct(name.to_string())
            }
            // PyO3 module types (np.ndarray, pd.DataFrame, etc.)
            name if name.contains('.') => Type::Any,
            _ => Type::Unknown,
        }
    }

    /// Convert to Rust type string
    pub fn to_rust_string(&self) -> String {
        match self {
            Type::Int => "i64".to_string(),
            Type::Float => "f64".to_string(),
            Type::String => "String".to_string(),
            Type::Bool => "bool".to_string(),
            Type::List(inner) => format!("Vec<{}>", inner.to_rust_string()),
            Type::Tuple(types) => {
                let inner: Vec<_> = types.iter().map(|t| t.to_rust_string()).collect();
                format!("({})", inner.join(", "))
            }
            Type::Dict(k, v) => {
                format!(
                    "std::collections::HashMap<{}, {}>",
                    k.to_rust_string(),
                    v.to_rust_string()
                )
            }
            Type::Set(inner) => {
                format!("std::collections::HashSet<{}>", inner.to_rust_string())
            }
            Type::Optional(inner) => {
                // For struct types, use Box to avoid infinite size
                if let Type::Struct(_) = inner.as_ref() {
                    format!("Option<Box<{}>>", inner.to_rust_string())
                } else {
                    format!("Option<{}>", inner.to_rust_string())
                }
            }
            Type::Ref(inner) => {
                // For List types, emit &[T] slice instead of &Vec<T> (more idiomatic)
                if let Type::List(elem_type) = inner.as_ref() {
                    format!("&[{}]", elem_type.to_rust_string())
                } else if let Type::String = inner.as_ref() {
                    "&str".to_string()
                } else {
                    format!("&{}", inner.to_rust_string())
                }
            }
            Type::MutRef(inner) => {
                // For List types, emit &mut [T] slice instead of &mut Vec<T> (more idiomatic)
                if let Type::List(elem_type) = inner.as_ref() {
                    format!("&mut [{}]", elem_type.to_rust_string())
                } else {
                    format!("&mut {}", inner.to_rust_string())
                }
            }
            Type::Func {
                params,
                ret,
                is_boxed,
                ..
            } => {
                let p: Vec<_> = params.iter().map(|t| t.to_rust_string()).collect();
                if *is_boxed {
                    // Use Arc<dyn Fn(...) + Send + Sync> for Clone support
                    format!(
                        "std::sync::Arc<dyn Fn({}) -> {} + Send + Sync>",
                        p.join(", "),
                        ret.to_rust_string()
                    )
                } else {
                    // Use fn(...) -> ... for raw function pointers (items)
                    format!("fn({}) -> {}", p.join(", "), ret.to_rust_string())
                }
            }
            Type::Unit => "()".to_string(),
            Type::Struct(name) => name.clone(),
            Type::Any => "tsuchinoko::bridge::protocol::TnkValue".to_string(),
            Type::Unknown => "tsuchinoko::bridge::protocol::TnkValue".to_string(),
        }
    }

    /// Check if type is Copy
    pub fn is_copy(&self) -> bool {
        matches!(
            self,
            Type::Int | Type::Float | Type::Bool | Type::Unit | Type::Ref(_) | Type::Any
        )
    }

    /// Check if this type is compatible with another type (considering Unknown as wildcard)
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        if self == other
            || *self == Type::Unknown
            || *other == Type::Unknown
            || *self == Type::Any
            || *other == Type::Any
        {
            return true;
        }

        match (self, other) {
            (Type::List(a), Type::List(b)) => a.is_compatible_with(b),
            (Type::Tuple(a), Type::Tuple(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                a.iter().zip(b.iter()).all(|(x, y)| x.is_compatible_with(y))
            }
            (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
                k1.is_compatible_with(k2) && v1.is_compatible_with(v2)
            }
            (Type::Set(a), Type::Set(b)) => a.is_compatible_with(b),
            (Type::Optional(a), Type::Optional(b)) => a.is_compatible_with(b),
            (Type::Ref(a), Type::Ref(b)) => a.is_compatible_with(b),
            (
                Type::Func {
                    params: p1,
                    ret: r1,
                    ..
                },
                Type::Func {
                    params: p2,
                    ret: r2,
                    ..
                },
            ) => {
                if p1.len() != p2.len() {
                    return false;
                }
                p1.iter()
                    .zip(p2.iter())
                    .all(|(x, y)| x.is_compatible_with(y))
                    && r1.is_compatible_with(r2)
            }
            _ => false,
        }
    }

    /// Check if this type or any of its sub-types is Type::Unknown
    pub fn contains_unknown(&self) -> bool {
        match self {
            Type::Unknown => true,
            Type::List(inner) => inner.contains_unknown(),
            Type::Tuple(types) => types.iter().any(|t| t.contains_unknown()),
            Type::Dict(k, v) => k.contains_unknown() || v.contains_unknown(),
            Type::Set(inner) => inner.contains_unknown(),
            Type::Optional(inner) => inner.contains_unknown(),
            Type::Ref(inner) => inner.contains_unknown(),
            Type::MutRef(inner) => inner.contains_unknown(),
            Type::Func { params, ret, .. } => {
                params.iter().any(|t| t.contains_unknown()) || ret.contains_unknown()
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_from_python_hint_int() {
        let ty = Type::from_python_hint("int", &[]);
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_type_from_python_hint_list() {
        let ty = Type::from_python_hint("list", &[Type::Int]);
        assert_eq!(ty, Type::List(Box::new(Type::Int)));
    }

    #[test]
    fn test_type_to_rust_string() {
        assert_eq!(Type::Int.to_rust_string(), "i64");
        assert_eq!(Type::String.to_rust_string(), "String");
        assert_eq!(Type::List(Box::new(Type::Int)).to_rust_string(), "Vec<i64>");
    }
}
