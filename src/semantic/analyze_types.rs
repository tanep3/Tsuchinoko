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
                if let Expr::Attribute { value, .. } = func.as_ref() {
                    if let Expr::Ident(module_alias) = value.as_ref() {
                        let is_pyo3_module = self
                            .external_imports
                            .iter()
                            .any(|(_, alias)| alias == module_alias);
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
                // If target is Type::Any, attribute access returns Type::Any
                let target_ty = self.infer_type(value);
                if matches!(target_ty, Type::Any) {
                    return Type::Any;
                }
                Type::Unknown
            }
            _ => Type::Unknown,
        }
    }
}
