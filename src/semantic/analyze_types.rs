//! Type inference implementation for SemanticAnalyzer
//!
//! Extracted from mod.rs for maintainability

use super::*;

impl SemanticAnalyzer {
    pub(crate) fn infer_type(&self, expr: &Expr) -> Type {
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
                let ty = if let Some(info) = self.scope.lookup(name) {
                    info.ty.clone()
                } else {
                    Type::Unknown
                };
                ty
            }
            Expr::Index { target, index: _ } => {
                let target_ty = self.infer_type(target);
                if let Type::List(inner) = target_ty {
                    *inner
                } else if let Type::Ref(inner) | Type::MutRef(inner) = target_ty {
                    if let Type::List(elem) = *inner {
                        *elem
                    } else if let Type::Tuple(elems) = *inner {
                        // For tuple ref, we need the specific element type if index is constant
                        // But for simplicity, if it's mixed, return Unknown
                        if elems.windows(2).all(|w| w[0] == w[1]) && !elems.is_empty() {
                            elems[0].clone()
                        } else {
                            Type::Unknown
                        }
                    } else {
                        Type::Unknown
                    }
                } else if matches!(target_ty, Type::Any) {
                    Type::Any
                } else {
                    Type::Unknown
                }
            }
            Expr::Call { func, args, .. } => {
                // Check for PyO3 module calls first: np.*, pd.*, etc.
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Expr::Ident(module_name) = value.as_ref() {
                        // Resolve module alias (e.g., m -> math)
                        let real_module = self
                            .module_global_aliases
                            .get(module_name)
                            .map(|s| s.as_str())
                            .unwrap_or(module_name);

                        let full_target = format!("{real_module}.{attr}");
                        if crate::bridge::module_table::is_native_target(&full_target) {
                            return Type::Float;
                        }

                        let is_pyo3_module = self
                            .external_imports
                            .iter()
                            .any(|(_, alias)| alias == module_name);
                        if is_pyo3_module {
                            return Type::Any;
                        }
                    }
                    // Methods on Type::Any return Type::Any
                    let target_ty = self.infer_type(value);
                    if matches!(target_ty, Type::Any) {
                        return Type::Any;
                    }
                }

                // Try to resolve return type
                if let Expr::Ident(name) = func.as_ref() {
                    if name == "tuple" || name == "list" {
                        return Type::List(Box::new(Type::Unknown));
                    }
                    if name == "sorted" || name == "reversed" {
                        return Type::List(Box::new(Type::Unknown));
                    }
                    if name == "enumerate" {
                        return Type::List(Box::new(Type::Tuple(vec![Type::Int, Type::Unknown])));
                    }
                    if name == "zip" {
                        return Type::List(Box::new(Type::Tuple(vec![Type::Unknown; args.len()])));
                    }
                    if name == "map" || name == "filter" {
                        return Type::List(Box::new(Type::Unknown));
                    }
                    if name == "sum" {
                        return Type::Int;
                    }
                    if name == "all" || name == "any" {
                        return Type::Bool;
                    }
                    // dict(x) returns the same Dict type as x
                    if name == "dict" && !args.is_empty() {
                        let arg_ty = self.infer_type(&args[0]);
                        if let Type::Dict(k, v) = arg_ty {
                            return Type::Dict(k, v);
                        }
                        // If arg is unknown, still return Dict with unknown types
                        return Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown));
                    }
                    if let Some(info) = self.scope.lookup(name) {
                        if let Type::Func { params: _, ret, .. } = &info.ty {
                            return *ret.clone();
                        }
                    }
                    // V1.3.1: Check if this is a struct constructor call
                    if self.struct_field_types.contains_key(name) {
                        return Type::Struct(name.clone());
                    }
                    // Fallback for capitalized names (assumed to be structs even if defined elsewhere)
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        return Type::Struct(name.clone());
                    }
                } else if let Expr::Attribute { value, attr } = func.as_ref() {
                    let mut target_ty = self.infer_type(value);
                    while let Type::Ref(inner) = target_ty {
                        target_ty = *inner;
                    }
                    match (target_ty, attr.as_str()) {
                        (Type::Dict(k, v), "items") => {
                            return Type::List(Box::new(Type::Tuple(vec![*k.clone(), *v.clone()])))
                        }
                        (Type::Ref(inner), "items")
                            if matches!(inner.as_ref(), Type::Dict(_, _)) =>
                        {
                            if let Type::Dict(k, v) = inner.as_ref() {
                                return Type::List(Box::new(Type::Tuple(vec![
                                    *k.clone(),
                                    *v.clone(),
                                ])));
                            }
                        }
                        (Type::Dict(k, _), "keys") => return Type::List(k.clone()),
                        (Type::Ref(inner), "keys")
                            if matches!(inner.as_ref(), Type::Dict(_, _)) =>
                        {
                            if let Type::Dict(k, _) = inner.as_ref() {
                                return Type::List(k.clone());
                            }
                        }
                        (Type::Dict(_, v), "values") => return Type::List(v.clone()),
                        (Type::Ref(inner), "values")
                            if matches!(inner.as_ref(), Type::Dict(_, _)) =>
                        {
                            if let Type::Dict(_, v) = inner.as_ref() {
                                return Type::List(v.clone());
                            }
                        }
                        (Type::List(inner), "iter") => {
                            return Type::List(Box::new(Type::Ref(inner.clone())))
                        }
                        (Type::Ref(inner), "iter") => {
                            if let Type::List(elem) = inner.as_ref() {
                                return Type::List(Box::new(Type::Ref(elem.clone())));
                            } else if let Type::Dict(k, v) = inner.as_ref() {
                                return Type::List(Box::new(Type::Tuple(vec![
                                    Type::Ref(k.clone()),
                                    Type::Ref(v.clone()),
                                ])));
                            }
                        }
                        (Type::Dict(k, v), "iter") => {
                            return Type::List(Box::new(Type::Tuple(vec![
                                Type::Ref(k.clone()),
                                Type::Ref(v.clone()),
                            ])))
                        }
                        (Type::String, "join") => return Type::String,
                        _ => {}
                    }

                    // Check if this is a PyO3 module call (np.array, pd.DataFrame, etc.)
                    if let Expr::Ident(module_alias) = value.as_ref() {
                        if self
                            .external_imports
                            .iter()
                            .any(|(_, alias)| alias == module_alias)
                        {
                            return Type::Any;
                        }
                    }
                }
                Type::Unknown
            }
            Expr::BinOp { left, op, right: _ } => match op {
                AstBinOp::Add
                | AstBinOp::Sub
                | AstBinOp::Mul
                | AstBinOp::Div
                | AstBinOp::FloorDiv
                | AstBinOp::Mod
                | AstBinOp::Pow => self.infer_type(left),
                // Matrix multiplication returns left side type (V1.3.0)
                AstBinOp::MatMul => self.infer_type(left),
                // Bitwise operators return Int (V1.3.0)
                AstBinOp::BitAnd
                | AstBinOp::BitOr
                | AstBinOp::BitXor
                | AstBinOp::Shl
                | AstBinOp::Shr => Type::Int,
                // Comparison and logical operators return Bool
                AstBinOp::Eq
                | AstBinOp::NotEq
                | AstBinOp::Lt
                | AstBinOp::Gt
                | AstBinOp::LtEq
                | AstBinOp::GtEq
                | AstBinOp::And
                | AstBinOp::Or
                | AstBinOp::In
                | AstBinOp::NotIn  // V1.3.0
                | AstBinOp::Is
                | AstBinOp::IsNot => Type::Bool,
            },
            Expr::UnaryOp { op, operand } => match op {
                AstUnaryOp::Neg | AstUnaryOp::Pos => self.infer_type(operand),
                AstUnaryOp::Not => Type::Bool,
                AstUnaryOp::BitNot => Type::Int, // V1.3.0
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
            Expr::Attribute { value, attr } => {
                // Handle self.field type inference
                if let Expr::Ident(target_name) = value.as_ref() {
                    if target_name == "self" {
                        // Strip dunder prefix for lookup consistency
                        let rust_field = if attr.starts_with("__") && !attr.ends_with("__") {
                            attr.trim_start_matches("__")
                        } else {
                            attr.as_str()
                        };
                        // Look up self.field_name in scope
                        if let Some(info) = self.scope.lookup(&format!("self.{rust_field}")) {
                            return info.ty.clone();
                        }
                    }
                }

                // V1.4.0: Handle native module attributes (math.pi, etc.)
                if let Expr::Ident(module_name) = value.as_ref() {
                    let real_module = self
                        .module_global_aliases
                        .get(module_name)
                        .map(|s| s.as_str())
                        .unwrap_or(module_name);

                    let full_target = format!("{real_module}.{attr}");
                    if crate::bridge::module_table::is_native_target(&full_target) {
                        return Type::Float;
                    }
                }

                // If target is Type::Any, attribute access returns Type::Any
                let target_ty = self.infer_type(value);
                if matches!(target_ty, Type::Any) {
                    return Type::Any;
                }
                Type::Unknown
            }
            Expr::FString { .. } => Type::String,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Expr;

    #[test]
    fn test_infer_type_index_any_returns_any() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("v", Type::Any, false);
        let expr = Expr::Index {
            target: Box::new(Expr::Ident("v".to_string())),
            index: Box::new(Expr::IntLiteral(0)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Any);
    }

    #[test]
    fn test_infer_type_tuple_ref_uniform() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define(
            "t",
            Type::Ref(Box::new(Type::Tuple(vec![Type::Int, Type::Int]))),
            false,
        );
        let expr = Expr::Index {
            target: Box::new(Expr::Ident("t".to_string())),
            index: Box::new(Expr::IntLiteral(1)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Int);
    }

    #[test]
    fn test_infer_type_external_module_call_any() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.external_imports.push(("numpy".to_string(), "np".to_string()));
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("np".to_string())),
                attr: "array".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Any);
    }

    #[test]
    fn test_infer_type_native_module_call_float() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.module_global_aliases.insert("math".to_string(), "math".to_string());
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("math".to_string())),
                attr: "sqrt".to_string(),
            }),
            args: vec![Expr::FloatLiteral(4.0)],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Float);
    }

    #[test]
    fn test_infer_type_dict_call_from_dict_arg() {
        let mut analyzer = SemanticAnalyzer::new();
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
    fn test_infer_type_list_empty_is_list_unknown() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::List(vec![]);
        assert_eq!(analyzer.infer_type(&expr), Type::List(Box::new(Type::Unknown)));
    }

    #[test]
    fn test_infer_type_list_comp() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::ListComp {
            elt: Box::new(Expr::IntLiteral(1)),
            target: "x".to_string(),
            iter: Box::new(Expr::List(vec![Expr::IntLiteral(1)])),
            condition: None,
        };
        assert_eq!(analyzer.infer_type(&expr), Type::List(Box::new(Type::Int)));
    }

    #[test]
    fn test_infer_type_set_comp() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::SetComp {
            elt: Box::new(Expr::IntLiteral(1)),
            target: "x".to_string(),
            iter: Box::new(Expr::List(vec![Expr::IntLiteral(1)])),
            condition: None,
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Set(Box::new(Type::Int)));
    }

    #[test]
    fn test_infer_type_call_any_method_returns_any() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("v", Type::Any, false);
        let expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("v".to_string())),
                attr: "foo".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Any);
    }

    #[test]
    fn test_infer_type_call_sorted_returns_list() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("sorted".to_string())),
            args: vec![Expr::List(vec![Expr::IntLiteral(1)])],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::List(Box::new(Type::Unknown)));
    }

    #[test]
    fn test_infer_type_call_map_returns_list() {
        let analyzer = SemanticAnalyzer::new();
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
        assert_eq!(analyzer.infer_type(&expr), Type::List(Box::new(Type::Unknown)));
    }

    #[test]
    fn test_infer_type_call_filter_returns_list() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::Call {
            func: Box::new(Expr::Ident("filter".to_string())),
            args: vec![
                Expr::Lambda {
                    params: vec!["x".to_string()],
                    body: Box::new(Expr::BoolLiteral(true)),
                },
                Expr::List(vec![Expr::IntLiteral(1)]),
            ],
            kwargs: vec![],
        };
        assert_eq!(analyzer.infer_type(&expr), Type::List(Box::new(Type::Unknown)));
    }

    #[test]
    fn test_infer_type_index_list_returns_elem() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("xs", Type::List(Box::new(Type::Int)), false);
        let expr = Expr::Index {
            target: Box::new(Expr::Ident("xs".to_string())),
            index: Box::new(Expr::IntLiteral(0)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Int);
    }

    #[test]
    fn test_infer_type_index_tuple_mixed_unknown() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define(
            "t",
            Type::Ref(Box::new(Type::Tuple(vec![Type::Int, Type::String]))),
            false,
        );
        let expr = Expr::Index {
            target: Box::new(Expr::Ident("t".to_string())),
            index: Box::new(Expr::IntLiteral(0)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Unknown);
    }
}
