//! Type definitions

/// Rust types
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,      // i64
    Float,    // f64
    String,   // String
    Bool,     // bool
    List(Box<Type>),           // Vec<T>
    Tuple(Vec<Type>),          // (T, U, ...)
    Dict(Box<Type>, Box<Type>), // HashMap<K, V>
    Optional(Box<Type>),        // Option<T>
    Ref(Box<Type>),             // &T
    Func {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Unit,     // ()
    Struct(String),  // User-defined struct
    Unknown,  // Not yet inferred
}

impl Type {
    /// Convert Python type hint to Rust type
    pub fn from_python_hint(name: &str, params: &[Type]) -> Self {
        match name {
            "int" => Type::Int,
            "float" => Type::Float,
            "str" => Type::String,
            "bool" => Type::Bool,
            "list" | "List" => {
                let inner = params.first().cloned().unwrap_or(Type::Unknown);
                Type::List(Box::new(inner))
            }
            "tuple" | "Tuple" => Type::Tuple(params.to_vec()),
            "dict" | "Dict" => {
                let key = params.first().cloned().unwrap_or(Type::Unknown);
                let val = params.get(1).cloned().unwrap_or(Type::Unknown);
                Type::Dict(Box::new(key), Box::new(val))
            }
            "Optional" => {
                let inner = params.first().cloned().unwrap_or(Type::Unknown);
                Type::Optional(Box::new(inner))
            }
            "None" => Type::Unit,
            // Check if it's a user-defined type (capitalized name)
            name if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) => {
                Type::Struct(name.to_string())
            }
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
                format!("std::collections::HashMap<{}, {}>", k.to_rust_string(), v.to_rust_string())
            }
            Type::Optional(inner) => format!("Option<{}>", inner.to_rust_string()),
            Type::Ref(inner) => {
                // For List types, emit &[T] slice instead of &Vec<T> (more idiomatic)
                if let Type::List(elem_type) = inner.as_ref() {
                    format!("&[{}]", elem_type.to_rust_string())
                } else {
                    format!("&{}", inner.to_rust_string())
                }
            }
            Type::Func { params, ret } => {
                let p: Vec<_> = params.iter().map(|t| t.to_rust_string()).collect();
                format!("fn({}) -> {}", p.join(", "), ret.to_rust_string())
            }
            Type::Unit => "()".to_string(),
            Type::Struct(name) => name.clone(),
            Type::Unknown => "_".to_string(),
        }
    }

    /// Check if type is Copy
    pub fn is_copy(&self) -> bool {
        match self {
            Type::Int | Type::Float | Type::Bool | Type::Unit | Type::Ref(_) => true,
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
