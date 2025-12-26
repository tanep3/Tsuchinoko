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
    Func {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Unit,     // ()
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
                format!("HashMap<{}, {}>", k.to_rust_string(), v.to_rust_string())
            }
            Type::Optional(inner) => format!("Option<{}>", inner.to_rust_string()),
            Type::Func { params, ret } => {
                let p: Vec<_> = params.iter().map(|t| t.to_rust_string()).collect();
                format!("fn({}) -> {}", p.join(", "), ret.to_rust_string())
            }
            Type::Unit => "()".to_string(),
            Type::Unknown => "_".to_string(),
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
