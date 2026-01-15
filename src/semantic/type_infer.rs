//! Type inference module
//!
//! 型推論に関するロジックを提供する。
//! - `infer_type`: 式から型を推論
//! - `type_from_hint`: 型ヒントから型を生成
//! - `resolve_type`: 型エイリアスの解決
//! - `expr_to_type`: 式を型として解釈
//! - `resolve_type_with_context`: コンテキスト付き型解決 (V1.5.2)

use crate::parser::{BinOp as AstBinOp, Expr, TypeHint, UnaryOp as AstUnaryOp};

use super::{ScopeStack, Type};

/// 型解決コンテキスト
///
/// 「型を問う」操作を統一的に扱うための抽象化。
/// 同じ式でも、使用されるコンテキストによって異なる型が必要になる。
#[derive(Debug, Clone, PartialEq)]
pub enum TypeContext {
    /// 式の値としての型
    Value,
    /// イテラブルの要素としての型 (for loop)
    IterElement,
    /// 関数呼び出しの戻り値としての型
    CallReturn,
    /// インデックスアクセスの結果としての型
    Index,
    /// タプル展開の要素としての型 (index 指定)
    TupleUnpack(usize),
}

/// 型推論を行うトレイト
///
/// SemanticAnalyzerに実装される型推論機能を定義。
/// 他のモジュールからも利用可能にするためトレイトとして分離。
pub trait TypeInference {
    /// スコープへのアクセスを提供
    fn scope(&self) -> &ScopeStack;

    /// 外部モジュールインポート情報へのアクセス
    fn external_imports(&self) -> &[(String, String)];

    /// 型ヒントから Type を生成する
    ///
    /// # Arguments
    /// * `hint` - パーサーが生成した TypeHint
    ///
    /// # Returns
    /// 対応する Type
    fn type_from_hint(&self, hint: &TypeHint) -> Type {
        let params: Vec<Type> = hint.params.iter().map(|h| self.type_from_hint(h)).collect();
        Type::from_python_hint(&hint.name, &params)
    }

    /// 式の型を推論する
    ///
    /// # Arguments
    /// * `expr` - 型を推論する対象の式
    ///
    /// # Returns
    /// 推論された型。推論できない場合は Type::Unknown
    fn infer_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::IntLiteral(_) => Type::Int,
            Expr::FloatLiteral(_) => Type::Float,
            Expr::StringLiteral(_) => Type::String,
            Expr::BoolLiteral(_) => Type::Bool,

            Expr::List(elements) => {
                if let Some(first) = elements.first() {
                    Type::List(Box::new(self.infer_type(first)))
                } else {
                    Type::List(Box::new(Type::Unknown))
                }
            }

            Expr::ListComp { elt, .. } | Expr::GenExpr { elt, .. } => {
                Type::List(Box::new(self.infer_type(elt)))
            }

            // V1.6.0: Set comprehension type inference
            Expr::SetComp { elt, .. } => Type::Set(Box::new(self.infer_type(elt))),

            Expr::Ident(name) => {
                if let Some(info) = self.scope().lookup(name) {
                    info.ty.clone()
                } else if self
                    .external_imports()
                    .iter()
                    .any(|(_, alias)| alias == name)
                {
                    // V1.4.0 / V1.7.0: from-import された外部ライブラリの関数等は Type::Any とする
                    Type::Any
                } else {
                    Type::Unknown
                }
            }

            Expr::Index { target, .. } => self.infer_index_type(target),

            Expr::Call { func, args, .. } => self.infer_call_type(func, args),

            Expr::BinOp { left, op, right: _ } => self.infer_binop_type(left, op),

            Expr::UnaryOp { op, operand } => match op {
                AstUnaryOp::Neg | AstUnaryOp::Pos => self.infer_type(operand),
                AstUnaryOp::Not => Type::Bool,
                AstUnaryOp::BitNot => Type::Int,
            },

            Expr::IfExp { body, orelse, .. } => {
                let t_body = self.infer_type(body);
                let t_orelse = self.infer_type(orelse);
                if t_body == t_orelse {
                    t_body
                } else if t_body == Type::Unknown {
                    t_orelse
                } else if t_orelse == Type::Unknown {
                    t_body
                } else {
                    Type::Unknown
                }
            }

            Expr::Attribute { value, attr } => self.infer_attribute_type(value, attr),
            Expr::Lambda { params, body } => {
                let ret_ty = self.infer_type(body);
                Type::Func {
                    params: vec![Type::Unknown; params.len()],
                    ret: Box::new(ret_ty),
                    is_boxed: true,
                    may_raise: false,
                }
            }
            _ => Type::Unknown,
        }
    }

    /// インデックスアクセスの型を推論
    fn infer_index_type(&self, target: &Expr) -> Type {
        let target_ty = self.infer_type(target);
        match target_ty {
            Type::List(inner) => *inner,
            Type::Ref(inner) | Type::MutRef(inner) => match *inner {
                Type::List(elem) => *elem,
                Type::Tuple(elems) => {
                    // タプルの全要素が同じ型の場合はその型を返す
                    if elems.windows(2).all(|w| w[0] == w[1]) && !elems.is_empty() {
                        elems[0].clone()
                    } else {
                        Type::Unknown
                    }
                }
                _ => Type::Unknown,
            },
            _ => Type::Unknown,
        }
    }

    /// 関数呼び出しの戻り値型を推論
    fn infer_call_type(&self, func: &Expr, args: &[Expr]) -> Type {
        // PyO3モジュール呼び出しを先にチェック
        if let Expr::Attribute { value, .. } = func {
            if let Expr::Ident(module_alias) = value.as_ref() {
                let is_pyo3 = self
                    .external_imports()
                    .iter()
                    .any(|(_, alias)| alias == module_alias);
                if is_pyo3 {
                    return Type::Any;
                }
            }
            // Type::Any のメソッド呼び出しは Type::Any を返す
            if matches!(self.infer_type(value), Type::Any) {
                return Type::Any;
            }
        }

        // 関数名ベースの推論
        if let Expr::Ident(name) = func {
            if name == "dict" && !args.is_empty() {
                let arg_ty = self.infer_type(&args[0]);
                if let Type::Dict(k, v) = arg_ty {
                    return Type::Dict(k, v);
                }
                return Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown));
            }
            if let Some(spec) = crate::bridge::builtin_table::get_builtin_spec(name) {
                let arg_types: Vec<Type> = args.iter().map(|a| self.infer_type(a)).collect();
                return (spec.ret_ty_resolver)(&arg_types);
            }
            match name.as_str() {
                "str" => return Type::String,
                "int" => return Type::Int,
                "float" => return Type::Float,
                "bool" => return Type::Bool,
                "tuple" | "list" => return Type::List(Box::new(Type::Unknown)),
                "sorted" => return Type::List(Box::new(Type::Unknown)),
                "reversed" => return Type::List(Box::new(Type::Unknown)),
                "enumerate" => {
                    return Type::List(Box::new(Type::Tuple(vec![Type::Int, Type::Unknown])))
                }
                "zip" => return Type::List(Box::new(Type::Tuple(vec![Type::Unknown; args.len()]))),
                "map" => return Type::List(Box::new(Type::Unknown)),
                "filter" => return Type::List(Box::new(Type::Unknown)),
                "sum" => return Type::Int,
                "all" | "any" => return Type::Bool,
                _ => {
                    if let Some(info) = self.scope().lookup(name) {
                        if let Type::Func { ret, .. } = &info.ty {
                            return *ret.clone();
                        }
                    }
                }
            }
        }

        // メソッド呼び出しの型推論
        if let Expr::Attribute { value, attr } = func {
            return self.infer_method_return_type(value, attr);
        }

        Type::Unknown
    }

    /// メソッド呼び出しの戻り値型を推論
    fn infer_method_return_type(&self, target: &Expr, method: &str) -> Type {
        let mut target_ty = self.infer_type(target);

        // Ref をアンラップ
        while let Type::Ref(inner) = target_ty {
            target_ty = *inner;
        }

        match (&target_ty, method) {
            (Type::Dict(k, v), "items") => {
                Type::List(Box::new(Type::Tuple(vec![*k.clone(), *v.clone()])))
            }
            (Type::Dict(k, _), "keys") => Type::List(k.clone()),
            (Type::Dict(_, v), "values") => Type::List(v.clone()),
            (Type::List(inner), "iter") => Type::List(Box::new(Type::Ref(inner.clone()))),
            (Type::Dict(k, v), "iter") => Type::List(Box::new(Type::Tuple(vec![
                Type::Ref(k.clone()),
                Type::Ref(v.clone()),
            ]))),
            (Type::String, "join") => Type::String,
            (Type::List(inner), "pop") => *inner.clone(),
            (Type::List(_), "count" | "index") => Type::Int,
            (Type::Dict(_, v), "get" | "pop") => *v.clone(),
            _ => {
                // PyO3モジュール呼び出しをチェック
                if let Expr::Ident(module_alias) = target {
                    if self
                        .external_imports()
                        .iter()
                        .any(|(_, alias)| alias == module_alias)
                    {
                        return Type::Any;
                    }
                }
                Type::Unknown
            }
        }
    }

    /// 二項演算の結果型を推論
    fn infer_binop_type(&self, left: &Expr, op: &AstBinOp) -> Type {
        match op {
            AstBinOp::Add
            | AstBinOp::Sub
            | AstBinOp::Mul
            | AstBinOp::Div
            | AstBinOp::FloorDiv
            | AstBinOp::Mod
            | AstBinOp::Pow
            | AstBinOp::MatMul => self.infer_type(left),

            AstBinOp::BitAnd
            | AstBinOp::BitOr
            | AstBinOp::BitXor
            | AstBinOp::Shl
            | AstBinOp::Shr => Type::Int,

            AstBinOp::Eq
            | AstBinOp::NotEq
            | AstBinOp::Lt
            | AstBinOp::Gt
            | AstBinOp::LtEq
            | AstBinOp::GtEq
            | AstBinOp::And
            | AstBinOp::Or
            | AstBinOp::In
            | AstBinOp::NotIn
            | AstBinOp::Is
            | AstBinOp::IsNot => Type::Bool,
        }
    }

    /// 属性アクセスの型を推論
    fn infer_attribute_type(&self, value: &Expr, attr: &str) -> Type {
        if let Expr::Ident(target_name) = value {
            if target_name == "self" {
                // dunderプレフィックスを除去
                let rust_field = if attr.starts_with("__") && !attr.ends_with("__") {
                    attr.trim_start_matches("__")
                } else {
                    attr
                };
                if let Some(info) = self.scope().lookup(&format!("self.{rust_field}")) {
                    return info.ty.clone();
                }
            }
        }
        Type::Unknown
    }

    /// 型エイリアスを解決する
    ///
    /// Struct型の場合、スコープから実際の型を検索して解決する。
    fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Struct(name) => {
                if let Some(info) = self.scope().lookup(name) {
                    // 無限再帰を防止
                    if let Type::Struct(resolved_name) = &info.ty {
                        if resolved_name == name {
                            return ty.clone();
                        }
                    }
                    return self.resolve_type(&info.ty);
                }
                ty.clone()
            }
            Type::Ref(inner) => self.resolve_type(inner),
            _ => ty.clone(),
        }
    }

    /// V1.5.2: コンテキストに基づいて型を変換する
    ///
    /// 型情報の「伝播チェーン」を抽象化。
    /// 同じ型でもコンテキストによって異なる結果型が必要になる。
    fn apply_context(&self, ty: Type, context: &TypeContext) -> Type {
        match (ty.clone(), context) {
            // 値としての型: そのまま返す
            (t, TypeContext::Value) => t,

            // イテラブルの要素型を抽出
            (Type::List(inner), TypeContext::IterElement) => *inner,
            (Type::Ref(inner), TypeContext::IterElement) => self.apply_context(*inner, context),
            (Type::Dict(k, _), TypeContext::IterElement) => *k,
            (Type::String, TypeContext::IterElement) => Type::String,
            (Type::Set(inner), TypeContext::IterElement) => *inner,

            // 関数呼び出しの戻り値型を抽出
            (Type::Func { ret, .. }, TypeContext::CallReturn) => *ret,

            // インデックスアクセスの結果型
            (Type::List(inner), TypeContext::Index) => *inner,
            (Type::Dict(_, v), TypeContext::Index) => *v,
            (Type::Tuple(elems), TypeContext::Index) => {
                // タプルの全要素が同じ型の場合はその型を返す
                if !elems.is_empty() && elems.windows(2).all(|w| w[0] == w[1]) {
                    elems[0].clone()
                } else {
                    Type::Unknown
                }
            }

            // タプル展開の指定インデックスの要素
            (Type::Tuple(elems), TypeContext::TupleUnpack(i)) => {
                elems.get(*i).cloned().unwrap_or(Type::Unknown)
            }

            // 解決できない場合
            (_, _) => Type::Unknown,
        }
    }

    /// V1.5.2: コンテキスト付きで型を解決する
    ///
    /// 式の型を推論し、コンテキストに基づいて変換する。
    fn resolve_type_with_context(&self, expr: &Expr, context: &TypeContext) -> Type {
        let base_type = self.infer_type(expr);
        self.apply_context(base_type, context)
    }

    /// 式を型として解釈する（型エイリアス用）
    ///
    /// `ConditionFunction = Callable[[int], bool]` のような
    /// 型エイリアス定義で使用する。
    fn expr_to_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Ident(name) => Some(self.type_from_hint(&TypeHint {
                name: name.clone(),
                params: vec![],
            })),
            Expr::Index { target, index } => {
                if let Expr::Ident(name) = target.as_ref() {
                    match name.as_str() {
                        "Callable" => self.parse_callable_type(index),
                        "Dict" | "dict" => self.parse_dict_type(index),
                        "List" | "list" => {
                            let inner = self.expr_to_type(index)?;
                            Some(Type::List(Box::new(inner)))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Callable型をパース
    fn parse_callable_type(&self, index: &Expr) -> Option<Type> {
        if let Expr::Tuple(elements) = index {
            if elements.len() >= 2 {
                let param_list_expr = &elements[0];
                let ret_expr = &elements[1];

                let mut param_types = Vec::new();
                if let Expr::List(p_elems) = param_list_expr {
                    for p in p_elems {
                        param_types.push(self.expr_to_type(p)?);
                    }
                } else if let Some(t) = self.expr_to_type(param_list_expr) {
                    param_types.push(t);
                }

                let ret_type = self.expr_to_type(ret_expr).unwrap_or(Type::Unknown);

                return Some(Type::Func {
                    params: param_types,
                    ret: Box::new(ret_type),
                    is_boxed: true,
                    may_raise: false,
                });
            }
        }
        None
    }

    /// Dict型をパース
    fn parse_dict_type(&self, index: &Expr) -> Option<Type> {
        if let Expr::Tuple(elements) = index {
            if elements.len() >= 2 {
                let key_ty = self.expr_to_type(&elements[0]).unwrap_or(Type::Unknown);
                let val_ty = self.expr_to_type(&elements[1]).unwrap_or(Type::Unknown);
                return Some(Type::Dict(Box::new(key_ty), Box::new(val_ty)));
            }
        }
        None
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;

    // テスト用のモック構造体
    struct MockAnalyzer {
        scope: ScopeStack,
        external_imports: Vec<(String, String)>,
    }

    impl MockAnalyzer {
        fn new() -> Self {
            Self {
                scope: ScopeStack::new(),
                external_imports: Vec::new(),
            }
        }
    }

    impl TypeInference for MockAnalyzer {
        fn scope(&self) -> &ScopeStack {
            &self.scope
        }

        fn external_imports(&self) -> &[(String, String)] {
            &self.external_imports
        }
    }

    #[test]
    fn test_infer_int_literal() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::IntLiteral(42);
        assert_eq!(analyzer.infer_type(&expr), Type::Int);
    }

    #[test]
    fn test_infer_float_literal() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::FloatLiteral(3.14);
        assert_eq!(analyzer.infer_type(&expr), Type::Float);
    }

    #[test]
    fn test_infer_string_literal() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::StringLiteral("hello".to_string());
        assert_eq!(analyzer.infer_type(&expr), Type::String);
    }

    #[test]
    fn test_infer_bool_literal() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::BoolLiteral(true);
        assert_eq!(analyzer.infer_type(&expr), Type::Bool);
    }

    #[test]
    fn test_infer_list() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::List(vec![Expr::IntLiteral(1), Expr::IntLiteral(2)]);
        assert_eq!(analyzer.infer_type(&expr), Type::List(Box::new(Type::Int)));
    }

    #[test]
    fn test_infer_empty_list() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::List(vec![]);
        assert_eq!(
            analyzer.infer_type(&expr),
            Type::List(Box::new(Type::Unknown))
        );
    }

    #[test]
    fn test_type_from_hint_int() {
        let analyzer = MockAnalyzer::new();
        let hint = TypeHint {
            name: "int".to_string(),
            params: vec![],
        };
        assert_eq!(analyzer.type_from_hint(&hint), Type::Int);
    }

    #[test]
    fn test_type_from_hint_list_int() {
        let analyzer = MockAnalyzer::new();
        let hint = TypeHint {
            name: "list".to_string(),
            params: vec![TypeHint {
                name: "int".to_string(),
                params: vec![],
            }],
        };
        assert_eq!(
            analyzer.type_from_hint(&hint),
            Type::List(Box::new(Type::Int))
        );
    }

    #[test]
    fn test_infer_binop_comparison() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(1)),
            op: AstBinOp::Lt,
            right: Box::new(Expr::IntLiteral(2)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Bool);
    }

    #[test]
    fn test_infer_binop_arithmetic() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(1)),
            op: AstBinOp::Add,
            right: Box::new(Expr::IntLiteral(2)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Int);
    }

    #[test]
    fn test_infer_call_sum_returns_int() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("sum".to_string())),
            args: vec![Expr::List(vec![Expr::IntLiteral(1)])],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Int);
    }

    #[test]
    fn test_infer_call_any_returns_bool() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("any".to_string())),
            args: vec![Expr::List(vec![Expr::BoolLiteral(true)])],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Bool);
    }

    #[test]
    fn test_infer_call_all_returns_bool() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("all".to_string())),
            args: vec![Expr::List(vec![Expr::BoolLiteral(true)])],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Bool);
    }

    #[test]
    fn test_infer_call_enumerate_returns_list_tuple() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("enumerate".to_string())),
            args: vec![Expr::List(vec![Expr::IntLiteral(1)])],
            kwargs: vec![],
        };
        assert_eq!(
            analyzer.infer_type(&expr),
            Type::List(Box::new(Type::Tuple(vec![Type::Int, Type::Int])))
        );
    }

    #[test]
    fn test_infer_call_zip_returns_list_tuple_len() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("zip".to_string())),
            args: vec![Expr::List(vec![]), Expr::List(vec![]), Expr::List(vec![])],
            kwargs: vec![],
        };
        assert_eq!(
            analyzer.infer_type(&expr),
            Type::List(Box::new(Type::Tuple(vec![Type::Unknown; 3])))
        );
    }

    #[test]
    fn test_infer_call_map_returns_list_unknown() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("map".to_string())),
            args: vec![
                Expr::Lambda {
                    params: vec!["x".to_string()],
                    body: Box::new(Expr::Ident("x".to_string())),
                },
                Expr::List(vec![Expr::IntLiteral(1)]),
            ],
            kwargs: vec![],
        };
        assert_eq!(
            analyzer.infer_type(&expr),
            Type::List(Box::new(Type::Unknown))
        );
    }

    #[test]
    fn test_infer_method_dict_keys_returns_list() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.scope.define(
            "d",
            Type::Dict(Box::new(Type::Int), Box::new(Type::String)),
            false,
        );
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("d".to_string())),
                attr: "keys".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::List(Box::new(Type::Int)));
    }

    #[test]
    fn test_infer_method_dict_values_returns_list() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.scope.define(
            "d",
            Type::Dict(Box::new(Type::Int), Box::new(Type::String)),
            false,
        );
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("d".to_string())),
                attr: "values".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        assert_eq!(
            analyzer.infer_type(&expr),
            Type::List(Box::new(Type::String))
        );
    }

    #[test]
    fn test_infer_method_list_pop_returns_elem() {
        let mut analyzer = MockAnalyzer::new();
        analyzer
            .scope
            .define("xs", Type::List(Box::new(Type::Int)), false);
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("xs".to_string())),
                attr: "pop".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Int);
    }

    #[test]
    fn test_infer_method_string_join_returns_string() {
        let analyzer = MockAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::StringLiteral(",".to_string())),
                attr: "join".to_string(),
            }),
            args: vec![Expr::List(vec![Expr::StringLiteral("a".to_string())])],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::String);
    }

    #[test]
    fn test_infer_index_tuple_uniform_types() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.scope.define(
            "t",
            Type::Ref(Box::new(Type::Tuple(vec![Type::Int, Type::Int]))),
            false,
        );
        let expr = Expr::Index {
            target: Box::new(Expr::Ident("t".to_string())),
            index: Box::new(Expr::IntLiteral(0)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Int);
    }

    #[test]
    fn test_infer_index_tuple_mixed_types() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.scope.define(
            "t",
            Type::Ref(Box::new(Type::Tuple(vec![Type::Int, Type::String]))),
            false,
        );
        let expr = Expr::Index {
            target: Box::new(Expr::Ident("t".to_string())),
            index: Box::new(Expr::IntLiteral(1)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Unknown);
    }

    #[test]
    fn test_infer_call_dict_returns_dict_type() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.scope.define(
            "d",
            Type::Dict(Box::new(Type::Int), Box::new(Type::String)),
            false,
        );
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("dict".to_string())),
            args: vec![Expr::Ident("d".to_string())],
            kwargs: vec![],
        };
        assert_eq!(
            analyzer.infer_type(&expr),
            Type::Dict(Box::new(Type::Int), Box::new(Type::String))
        );
    }

    #[test]
    fn test_infer_external_module_call_is_any() {
        let mut analyzer = MockAnalyzer::new();
        analyzer
            .external_imports
            .push(("numpy".to_string(), "np".to_string()));
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("np".to_string())),
                attr: "zeros".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Any);
    }

    #[test]
    fn test_infer_method_dict_iter() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.scope.define(
            "d",
            Type::Dict(Box::new(Type::Int), Box::new(Type::String)),
            false,
        );
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("d".to_string())),
                attr: "iter".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        assert_eq!(
            analyzer.infer_type(&expr),
            Type::List(Box::new(Type::Tuple(vec![
                Type::Ref(Box::new(Type::Int)),
                Type::Ref(Box::new(Type::String)),
            ])))
        );
    }
}
