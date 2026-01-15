//! Type definitions
use serde::{Deserialize, Serialize};

/// Rust types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
                may_raise,
                ..
            } => {
                let p: Vec<_> = params.iter().map(|t| t.to_rust_string()).collect();
                let ret_str = if *may_raise {
                    format!("Result<{}, TsuchinokoError>", ret.to_rust_string())
                } else {
                    ret.to_rust_string()
                };
                if *is_boxed {
                    // Use Rc<dyn Fn(...)> for boxed callables (type aliases, fields)
                    format!("std::rc::Rc<dyn Fn({}) -> {}>", p.join(", "), ret_str)
                } else {
                    // Use fn(...) -> ... for raw function pointers (items)
                    format!("fn({}) -> {}", p.join(", "), ret_str)
                }
            }
            Type::Unit => "()".to_string(),
            Type::Struct(name) => name.clone(),
            Type::Any => "TnkValue".to_string(),
            Type::Unknown => "TnkValue".to_string(),
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

    /// V1.7.0: Generate idiomatic Rust default value for this type
    /// This is used for safe fallback returns in TryBlock or complex flow.
    pub fn to_default_value(&self) -> String {
        match self {
            Type::Int => "0i64".to_string(),
            Type::Float => "0.0".to_string(),
            Type::String => "String::new()".to_string(),
            Type::Bool => "false".to_string(),
            Type::List(_) => "vec![]".to_string(),
            Type::Set(_) => "std::collections::HashSet::new()".to_string(),
            Type::Tuple(types) => {
                let inner: Vec<_> = types.iter().map(|t| t.to_default_value()).collect();
                format!("({})", inner.join(", "))
            }
            Type::Dict(_, _) => "std::collections::HashMap::new()".to_string(),
            Type::Optional(_) => "None".to_string(),
            Type::Any | Type::Unknown => "TnkValue::Value { value: None }".to_string(),
            Type::Unit => "()".to_string(),
            Type::Ref(inner) => {
                // Return a mock static ref if needed, but usually we shouldn't fall back to refs.
                // For simplicity, return default of inner (might require leak/static)
                // Actually, Tsuchinoko functions usually return owned values or Result.
                format!("&{}", inner.to_default_value())
            }
            Type::Struct(name) => {
                // Assumes Default trait or minimal init
                format!("{}::default()", name)
            }
            _ => "todo!(\"default value for this type\")".to_string(),
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

    #[test]
    fn test_type_from_python_hint_dict_default() {
        let ty = Type::from_python_hint("dict", &[]);
        assert_eq!(
            ty,
            Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown))
        );
    }

    #[test]
    fn test_type_from_python_hint_tuple_empty_returns_list_unknown() {
        let ty = Type::from_python_hint("tuple", &[]);
        assert_eq!(ty, Type::List(Box::new(Type::Unknown)));
    }

    #[test]
    fn test_type_from_python_hint_tuple_params() {
        let ty = Type::from_python_hint("tuple", &[Type::Int, Type::String]);
        assert_eq!(ty, Type::Tuple(vec![Type::Int, Type::String]));
    }

    #[test]
    fn test_type_from_python_hint_optional() {
        let ty = Type::from_python_hint("Optional", &[Type::Int]);
        assert_eq!(ty, Type::Optional(Box::new(Type::Int)));
    }

    #[test]
    fn test_type_from_python_hint_set() {
        let ty = Type::from_python_hint("set", &[Type::Int]);
        assert_eq!(ty, Type::Set(Box::new(Type::Int)));
    }

    #[test]
    fn test_type_from_python_hint_callable_list_params() {
        let ty = Type::from_python_hint("Callable", &[Type::List(Box::new(Type::Int)), Type::Bool]);
        if let Type::Func {
            params,
            ret,
            is_boxed,
            ..
        } = ty
        {
            assert_eq!(params, vec![Type::Int]);
            assert_eq!(*ret, Type::Bool);
            assert!(is_boxed);
        } else {
            panic!("Expected Func type");
        }
    }

    #[test]
    fn test_type_from_python_hint_callable_tuple_params() {
        let ty = Type::from_python_hint(
            "Callable",
            &[Type::Tuple(vec![Type::Int, Type::String]), Type::Float],
        );
        if let Type::Func {
            params,
            ret,
            is_boxed,
            ..
        } = ty
        {
            assert_eq!(params, vec![Type::Int, Type::String]);
            assert_eq!(*ret, Type::Float);
            assert!(is_boxed);
        } else {
            panic!("Expected Func type");
        }
    }

    #[test]
    fn test_type_from_python_hint_struct_name() {
        let ty = Type::from_python_hint("Point", &[]);
        assert_eq!(ty, Type::Struct("Point".to_string()));
    }

    #[test]
    fn test_type_from_python_hint_dotted_name_any() {
        let ty = Type::from_python_hint("np.ndarray", &[]);
        assert_eq!(ty, Type::Any);
    }

    #[test]
    fn test_type_to_rust_string_dict() {
        let ty = Type::Dict(Box::new(Type::Int), Box::new(Type::String));
        assert_eq!(
            ty.to_rust_string(),
            "std::collections::HashMap<i64, String>"
        );
    }

    #[test]
    fn test_type_to_rust_string_optional() {
        let ty = Type::Optional(Box::new(Type::Bool));
        assert_eq!(ty.to_rust_string(), "Option<bool>");
    }

    #[test]
    fn test_type_to_default_value_dict_optional_tuple() {
        let dict = Type::Dict(Box::new(Type::Int), Box::new(Type::String));
        let opt = Type::Optional(Box::new(Type::Int));
        let tup = Type::Tuple(vec![Type::Int, Type::Bool]);
        assert_eq!(dict.to_default_value(), "std::collections::HashMap::new()");
        assert_eq!(opt.to_default_value(), "None");
        assert_eq!(tup.to_default_value(), "(0i64, false)");
    }
}
