//! Built-in function handling module
//!
//! Python組み込み関数の処理を提供する。
//! テーブル駆動設計により、組み込み関数の追加が容易。

use super::Type;

/// 組み込み関数のメタデータ
#[derive(Debug, Clone)]
pub struct BuiltinInfo {
    /// 期待される引数の数 (None = 可変長)
    pub arg_count: Option<usize>,
    /// 戻り値の型
    pub return_type: Type,
    /// 説明
    pub description: &'static str,
}

/// 組み込み関数の情報を取得
///
/// # Arguments
/// * `name` - 関数名
///
/// # Returns
/// 関数情報。組み込み関数でない場合はNone
pub fn get_builtin_info(name: &str) -> Option<BuiltinInfo> {
    match name {
        "range" => Some(BuiltinInfo {
            arg_count: None, // 1-3 args
            return_type: Type::List(Box::new(Type::Int)),
            description: "Generate a range of integers",
        }),
        "len" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::Int,
            description: "Return the length of an object",
        }),
        "list" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::List(Box::new(Type::Unknown)),
            description: "Convert to list",
        }),
        "str" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::String,
            description: "Convert to string",
        }),
        "int" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::Int,
            description: "Convert to integer",
        }),
        "float" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::Float,
            description: "Convert to float",
        }),
        "bool" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::Bool,
            description: "Convert to boolean",
        }),
        "tuple" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::List(Box::new(Type::Unknown)),
            description: "Convert to tuple/list",
        }),
        "dict" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown)),
            description: "Create or convert to dictionary",
        }),
        "max" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::Unknown,
            description: "Return the maximum value",
        }),
        "min" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::Unknown,
            description: "Return the minimum value",
        }),
        "sum" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::Unknown, // depends on input
            description: "Sum all elements",
        }),
        "abs" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::Unknown, // same as input
            description: "Return absolute value",
        }),
        "enumerate" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::List(Box::new(Type::Tuple(vec![Type::Int, Type::Unknown]))),
            description: "Return enumerated pairs",
        }),
        "zip" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::List(Box::new(Type::Tuple(vec![]))),
            description: "Zip iterables together",
        }),
        "sorted" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::List(Box::new(Type::Unknown)),
            description: "Return sorted list",
        }),
        "reversed" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::List(Box::new(Type::Unknown)),
            description: "Return reversed iterator",
        }),
        "all" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::Bool,
            description: "Return True if all elements are truthy",
        }),
        "any" => Some(BuiltinInfo {
            arg_count: Some(1),
            return_type: Type::Bool,
            description: "Return True if any element is truthy",
        }),
        "map" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::List(Box::new(Type::Unknown)),
            description: "Apply function to all elements",
        }),
        "filter" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::List(Box::new(Type::Unknown)),
            description: "Filter elements by predicate",
        }),
        "print" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::Unit,
            description: "Print to stdout",
        }),
        "input" => Some(BuiltinInfo {
            arg_count: None,
            return_type: Type::String,
            description: "Read from stdin",
        }),
        _ => None,
    }
}

/// 組み込み関数かどうかを判定
///
/// # Arguments
/// * `name` - 関数名
///
/// # Returns
/// 組み込み関数の場合true
pub fn is_builtin(name: &str) -> bool {
    get_builtin_info(name).is_some()
}

/// 組み込み関数の戻り値型を取得
///
/// # Arguments
/// * `name` - 関数名
///
/// # Returns
/// 戻り値の型。組み込み関数でない場合はNone
pub fn get_builtin_return_type(name: &str) -> Option<Type> {
    get_builtin_info(name).map(|info| info.return_type)
}

/// 全組み込み関数のリストを取得
pub fn list_all_builtins() -> Vec<&'static str> {
    vec![
        "range",
        "len",
        "list",
        "str",
        "int",
        "float",
        "bool",
        "tuple",
        "dict",
        "max",
        "min",
        "sum",
        "abs",
        "enumerate",
        "zip",
        "sorted",
        "reversed",
        "all",
        "any",
        "map",
        "filter",
        "print",
        "input",
    ]
}

/// メソッドのメタデータ
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// 期待される引数の数
    pub arg_count: usize,
    /// 戻り値の型
    pub return_type: Type,
    /// mutationを起こすかどうか
    pub is_mutating: bool,
}

/// リストメソッドの情報を取得
///
/// # Arguments
/// * `method` - メソッド名
/// * `elem_type` - リストの要素型
///
/// # Returns
/// メソッド情報
pub fn get_list_method_info(method: &str, elem_type: &Type) -> Option<MethodInfo> {
    match method {
        "append" | "push" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        "extend" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        "pop" => Some(MethodInfo {
            arg_count: 0,
            return_type: elem_type.clone(),
            is_mutating: true,
        }),
        "insert" => Some(MethodInfo {
            arg_count: 2,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        "remove" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        "clear" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        "sort" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        "reverse" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        "copy" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::List(Box::new(elem_type.clone())),
            is_mutating: false,
        }),
        "len" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::Int,
            is_mutating: false,
        }),
        "index" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::Int,
            is_mutating: false,
        }),
        "count" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::Int,
            is_mutating: false,
        }),
        _ => None,
    }
}

/// Dictメソッドの情報を取得
pub fn get_dict_method_info(
    method: &str,
    key_type: &Type,
    value_type: &Type,
) -> Option<MethodInfo> {
    match method {
        "get" => Some(MethodInfo {
            arg_count: 1, // 1-2
            return_type: Type::Optional(Box::new(value_type.clone())),
            is_mutating: false,
        }),
        "keys" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::List(Box::new(key_type.clone())),
            is_mutating: false,
        }),
        "values" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::List(Box::new(value_type.clone())),
            is_mutating: false,
        }),
        "items" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::List(Box::new(Type::Tuple(vec![
                key_type.clone(),
                value_type.clone(),
            ]))),
            is_mutating: false,
        }),
        "pop" => Some(MethodInfo {
            arg_count: 1,
            return_type: value_type.clone(),
            is_mutating: true,
        }),
        "clear" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        "update" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::Unit,
            is_mutating: true,
        }),
        _ => None,
    }
}

/// Stringメソッドの情報を取得
pub fn get_string_method_info(method: &str) -> Option<MethodInfo> {
    match method {
        "lower" | "upper" | "strip" | "lstrip" | "rstrip" | "trim" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::String,
            is_mutating: false,
        }),
        "split" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::List(Box::new(Type::String)),
            is_mutating: false,
        }),
        "join" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::String,
            is_mutating: false,
        }),
        "replace" => Some(MethodInfo {
            arg_count: 2,
            return_type: Type::String,
            is_mutating: false,
        }),
        "startswith" | "endswith" | "contains" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::Bool,
            is_mutating: false,
        }),
        "len" => Some(MethodInfo {
            arg_count: 0,
            return_type: Type::Int,
            is_mutating: false,
        }),
        "find" | "index" => Some(MethodInfo {
            arg_count: 1,
            return_type: Type::Int,
            is_mutating: false,
        }),
        "format" => Some(MethodInfo {
            arg_count: 0, // variadic
            return_type: Type::String,
            is_mutating: false,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_true() {
        assert!(is_builtin("range"));
        assert!(is_builtin("len"));
        assert!(is_builtin("print"));
    }

    #[test]
    fn test_is_builtin_false() {
        assert!(!is_builtin("my_custom_func"));
        assert!(!is_builtin("foobar"));
    }

    #[test]
    fn test_get_builtin_info() {
        let info = get_builtin_info("len").unwrap();
        assert_eq!(info.arg_count, Some(1));
        assert_eq!(info.return_type, Type::Int);
    }

    #[test]
    fn test_get_builtin_return_type() {
        assert_eq!(get_builtin_return_type("len"), Some(Type::Int));
        assert_eq!(get_builtin_return_type("str"), Some(Type::String));
        assert_eq!(get_builtin_return_type("bool"), Some(Type::Bool));
    }

    #[test]
    fn test_list_all_builtins() {
        let builtins = list_all_builtins();
        assert!(builtins.contains(&"range"));
        assert!(builtins.contains(&"print"));
        assert!(!builtins.contains(&"not_a_builtin"));
    }

    #[test]
    fn test_get_list_method_info() {
        let info = get_list_method_info("append", &Type::Int).unwrap();
        assert_eq!(info.arg_count, 1);
        assert!(info.is_mutating);
    }

    #[test]
    fn test_get_list_method_info_copy() {
        let info = get_list_method_info("copy", &Type::String).unwrap();
        assert_eq!(info.return_type, Type::List(Box::new(Type::String)));
        assert!(!info.is_mutating);
    }

    #[test]
    fn test_get_dict_method_info() {
        let info = get_dict_method_info("keys", &Type::String, &Type::Int).unwrap();
        assert_eq!(info.return_type, Type::List(Box::new(Type::String)));
    }

    #[test]
    fn test_get_string_method_info() {
        let info = get_string_method_info("split").unwrap();
        assert_eq!(info.return_type, Type::List(Box::new(Type::String)));
    }
}
