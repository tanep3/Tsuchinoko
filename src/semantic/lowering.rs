//! Lowering Pass
//!
//! Semantic Analysis (型推論) の後、Emitter (コード生成) の前に行う
//! IR正規化・最適化パス。

use crate::ir::exprs::{IrExpr, IrExprKind, BuiltinId, ExprId};
use crate::ir::ops::IrBinOp;
use crate::ir::nodes::IrNode;
use crate::semantic::Type;
use crate::bridge::builtin_table::BuiltinKind;
use std::collections::HashMap;
use std::cell::Cell;

pub struct LoweringPass {
    module_aliases: HashMap<String, String>,
    type_table: HashMap<ExprId, Type>,
    next_id: Cell<u32>,
}

impl LoweringPass {
    pub fn new(module_aliases: HashMap<String, String>, type_table: HashMap<ExprId, Type>, next_id_start: u32) -> Self {
        Self { 
            module_aliases, 
            type_table, 
            next_id: Cell::new(next_id_start),
        }
    }

    fn next_id(&self) -> ExprId {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        ExprId(id)
    }

    /// IR全体に対してLoweringを実行
    pub fn apply(&self, ir: Vec<IrNode>) -> Vec<IrNode> {
        ir.into_iter().map(|node| self.lower_node(node)).collect()
    }

    fn lower_node(&self, node: IrNode) -> IrNode {
        match node {
            IrNode::FuncDecl {
                name, params, body, ret, hoisted_vars, may_raise, needs_bridge,
            } => {
                let new_body = body.into_iter().map(|n| self.lower_node(n)).collect();
                IrNode::FuncDecl {
                    name, params, body: new_body, ret, hoisted_vars, may_raise, needs_bridge,
                }
            }
            IrNode::MethodDecl {
                name, params, body, ret, takes_self, takes_mut_self, may_raise, needs_bridge,
            } => {
                let new_body = body.into_iter().map(|n| self.lower_node(n)).collect();
                IrNode::MethodDecl {
                    name, params, body: new_body, ret, takes_self, takes_mut_self, may_raise, needs_bridge,
                }
            }
            IrNode::ImplBlock { struct_name, methods } => {
                let new_methods = methods.into_iter().map(|m| self.lower_node(m)).collect();
                IrNode::ImplBlock { struct_name, methods: new_methods }
            }
            IrNode::Assign { target, value } => {
                 IrNode::Assign { target, value: Box::new(self.lower_expr(*value)) }
            }
            IrNode::VarDecl { name, ty, mutable, init } => IrNode::VarDecl {
                name, ty, mutable, init: init.map(|e| Box::new(self.lower_expr(*e))),
            },
            IrNode::Expr(expr) => IrNode::Expr(self.lower_expr(expr)),
            IrNode::Return(Some(expr)) => IrNode::Return(Some(Box::new(self.lower_expr(*expr)))),
            IrNode::If { cond, then_block, else_block } => IrNode::If {
                cond: Box::new(self.lower_expr(*cond)),
                then_block: then_block.into_iter().map(|n| self.lower_node(n)).collect(),
                else_block: else_block.map(|block| block.into_iter().map(|n| self.lower_node(n)).collect()),
            },
            IrNode::While { cond, body } => IrNode::While {
                cond: Box::new(self.lower_expr(*cond)),
                body: body.into_iter().map(|n| self.lower_node(n)).collect(),
            },
            IrNode::For { var, var_type, iter, body } => IrNode::For {
                var, var_type, iter: Box::new(self.lower_expr(*iter)),
                body: body.into_iter().map(|n| self.lower_node(n)).collect(),
            },
            IrNode::TryBlock { try_body, except_body, except_var, else_body, finally_body } => IrNode::TryBlock {
                try_body: try_body.into_iter().map(|n| self.lower_node(n)).collect(),
                except_body: except_body.into_iter().map(|n| self.lower_node(n)).collect(),
                except_var,
                else_body: else_body.map(|b| b.into_iter().map(|n| self.lower_node(n)).collect()),
                finally_body: finally_body.map(|b| b.into_iter().map(|n| self.lower_node(n)).collect()),
            },
            IrNode::Assert { test, msg } => IrNode::Assert {
                test: Box::new(self.lower_expr(*test)),
                msg: msg.map(|e| Box::new(self.lower_expr(*e))),
            },
            IrNode::Raise { exc_type, message, cause, line } => IrNode::Raise {
                exc_type,
                message: Box::new(self.lower_expr(*message)),
                cause: cause.map(|e| Box::new(self.lower_expr(*e))),
                line,
            },
            _ => node, 
        }
    }

    fn lower_expr(&self, expr: IrExpr) -> IrExpr {
        let id = expr.id;
        let kind = match expr.kind {
            IrExprKind::BuiltinCall { id: builtin_id, args } => {
                return self.lower_builtin_call(id, builtin_id, args);
            }
            IrExprKind::Var(name) => {
                if let Some(real_target) = self.module_aliases.get(&name) {
                    if !crate::bridge::module_table::is_native_module(real_target) {
                        IrExprKind::BridgeGet { alias: name }
                    } else {
                        IrExprKind::Var(name)
                    }
                } else {
                    IrExprKind::Var(name)
                }
            }
            IrExprKind::MethodCall { target, method, args, target_type, callee_needs_bridge } => {
                IrExprKind::MethodCall {
                    target: Box::new(self.lower_expr_as_target(*target)),
                    method,
                    args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                    target_type,
                    callee_needs_bridge,
                }
            }
            IrExprKind::BridgeMethodCall { target, method, args, keywords } => {
                IrExprKind::BridgeMethodCall {
                    target: Box::new(self.lower_expr_as_target(*target)),
                    method,
                    args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                    keywords: keywords.into_iter().map(|(k, v)| (k, self.lower_expr(v))).collect(),
                }
            }
            IrExprKind::BridgeAttributeAccess { target, attribute } => {
                IrExprKind::BridgeAttributeAccess {
                    target: Box::new(self.lower_expr_as_target(*target)),
                    attribute,
                }
            }
            IrExprKind::BridgeItemAccess { target, index } => {
                IrExprKind::BridgeItemAccess {
                    target: Box::new(self.lower_expr_as_target(*target)),
                    index: Box::new(self.lower_expr(*index)),
                }
            }
            IrExprKind::BridgeSlice { target, start, stop, step } => {
                IrExprKind::BridgeSlice {
                    target: Box::new(self.lower_expr_as_target(*target)),
                    start: Box::new(self.lower_expr(*start)),
                    stop: Box::new(self.lower_expr(*stop)),
                    step: Box::new(self.lower_expr(*step)),
                }
            }
            IrExprKind::PyO3MethodCall { target, method, args } => {
                IrExprKind::PyO3MethodCall {
                    target: Box::new(self.lower_expr_as_target(*target)),
                    method,
                    args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                }
            }
            IrExprKind::PyO3Call { module, method, args } => {
                IrExprKind::PyO3Call {
                    module,
                    method,
                    args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                }
            }
            IrExprKind::TnkValueFrom(inner) => {
                IrExprKind::TnkValueFrom(Box::new(self.lower_expr_as_target(*inner)))
            }
            IrExprKind::BinOp { left, op, right } => IrExprKind::BinOp {
                left: Box::new(self.lower_expr(*left)),
                op,
                right: Box::new(self.lower_expr(*right)),
            },
            IrExprKind::Call { func, args, callee_may_raise, callee_needs_bridge } => {
                let lowered_func = self.lower_expr_as_target(*func);
                if matches!(lowered_func.kind, IrExprKind::BridgeGet { .. }) {
                    let bridge_call = IrExpr {
                        id,
                        kind: IrExprKind::BridgeCall {
                            target: Box::new(lowered_func),
                            args: args.into_iter().map(|a| {
                                let lowered_arg = self.lower_expr(a);
                                IrExpr {
                                    id: self.next_id(),
                                    kind: IrExprKind::Ref(Box::new(IrExpr {
                                        id: self.next_id(),
                                        kind: IrExprKind::TnkValueFrom(Box::new(lowered_arg)),
                                    })),
                                }
                            }).collect(),
                            keywords: vec![],
                        },
                    };
                    if let Some(expected_ty) = self.type_table.get(&id) {
                        if *expected_ty != Type::Any && *expected_ty != Type::Unknown {
                            return IrExpr {
                                id,
                                kind: IrExprKind::FromTnkValue {
                                    value: Box::new(bridge_call),
                                    to_type: expected_ty.clone(),
                                },
                            };
                        }
                    }
                    return IrExpr { id, kind: bridge_call.kind };
                }
                IrExprKind::Call {
                    func: Box::new(lowered_func),
                    args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                    callee_may_raise,
                    callee_needs_bridge,
                }
            }
            IrExprKind::FieldAccess { target, field } => IrExprKind::FieldAccess { 
                target: Box::new(self.lower_expr(*target)), field 
            },
            IrExprKind::UnaryOp { op, operand } => IrExprKind::UnaryOp { 
                op, operand: Box::new(self.lower_expr(*operand)) 
            },
            IrExprKind::List { elem_type, elements } => IrExprKind::List { 
                elem_type, elements: elements.into_iter().map(|e| self.lower_expr(e)).collect() 
            },
            IrExprKind::Set { elem_type, elements } => IrExprKind::Set {
                elem_type, elements: elements.into_iter().map(|e| self.lower_expr(e)).collect()
            },
            IrExprKind::ListComp { elt, target, iter, condition } => IrExprKind::ListComp {
                elt: Box::new(self.lower_expr(*elt)),
                target,
                iter: Box::new(self.lower_expr(*iter)),
                condition: condition.map(|c| Box::new(self.lower_expr(*c))),
            },
            IrExprKind::SetComp { elt, target, iter, condition } => IrExprKind::SetComp {
                elt: Box::new(self.lower_expr(*elt)),
                target,
                iter: Box::new(self.lower_expr(*iter)),
                condition: condition.map(|c| Box::new(self.lower_expr(*c))),
            },
            IrExprKind::DictComp { key, value, target, iter, condition } => IrExprKind::DictComp {
                key: Box::new(self.lower_expr(*key)),
                value: Box::new(self.lower_expr(*value)),
                target,
                iter: Box::new(self.lower_expr(*iter)),
                condition: condition.map(|c| Box::new(self.lower_expr(*c))),
            },
            IrExprKind::Ref(inner) => IrExprKind::Ref(Box::new(self.lower_expr(*inner))),
            IrExprKind::Dict { key_type, value_type, entries } => IrExprKind::Dict {
                key_type, value_type,
                entries: entries.into_iter().map(|(k, v)| (self.lower_expr(k), self.lower_expr(v))).collect(),
            },
            IrExprKind::Tuple(elements) => IrExprKind::Tuple(elements.into_iter().map(|e| self.lower_expr(e)).collect()),
            IrExprKind::Index { target, index } => IrExprKind::Index { 
                target: Box::new(self.lower_expr(*target)), index: Box::new(self.lower_expr(*index)) 
            },
            IrExprKind::Slice { target, start, end, step } => IrExprKind::Slice {
                target: Box::new(self.lower_expr(*target)),
                start: start.map(|s| Box::new(self.lower_expr(*s))),
                end: end.map(|e| Box::new(self.lower_expr(*e))),
                step: step.map(|s| Box::new(self.lower_expr(*s))),
            },
            IrExprKind::FString { parts, values } => IrExprKind::FString {
                parts, values: values.into_iter().map(|(v, ty)| (self.lower_expr(v), ty)).collect(),
            },
            IrExprKind::Print { args } => IrExprKind::Print {
                args: args.into_iter().map(|(v, ty)| (self.lower_expr(v), ty)).collect(),
            },
            IrExprKind::IfExp { test, body, orelse } => IrExprKind::IfExp {
                test: Box::new(self.lower_expr(*test)),
                body: Box::new(self.lower_expr(*body)),
                orelse: Box::new(self.lower_expr(*orelse)),
            },
            IrExprKind::Closure { params, body, ret_type } => IrExprKind::Closure {
                params,
                body: body.into_iter().map(|n| self.lower_node(n)).collect(),
                ret_type,
            },
            IrExprKind::Unwrap(inner) => IrExprKind::Unwrap(Box::new(self.lower_expr(*inner))),
            IrExprKind::BoxNew(inner) => IrExprKind::BoxNew(Box::new(self.lower_expr(*inner))),
            IrExprKind::Reference { target } => IrExprKind::Reference {
                target: Box::new(self.lower_expr(*target)),
            },
            IrExprKind::MutReference { target } => IrExprKind::MutReference {
                target: Box::new(self.lower_expr(*target)),
            },
            IrExprKind::Cast { target, ty } => IrExprKind::Cast {
                target: Box::new(self.lower_expr(*target)),
                ty,
            },
            IrExprKind::JsonConversion { target, convert_to } => IrExprKind::JsonConversion {
                target: Box::new(self.lower_expr(*target)),
                convert_to,
            },
            IrExprKind::BridgeGet { alias } => IrExprKind::TnkValueFrom(Box::new(IrExpr { id: self.next_id(), kind: IrExprKind::BridgeGet { alias } })),
            IrExprKind::BridgeCall { target, args, keywords } => IrExprKind::BridgeCall {
                target: Box::new(self.lower_expr_as_target(*target)),
                args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                keywords: keywords.into_iter().map(|(k, v)| (k, self.lower_expr(v))).collect(),
            },
            _ => expr.kind,
        };
        IrExpr { id, kind }
    }

    fn lower_expr_as_target(&self, expr: IrExpr) -> IrExpr {
        let id = expr.id;
        let kind = match expr.kind {
            IrExprKind::BridgeGet { .. } => expr.kind,
            IrExprKind::Var(name) => {
                if let Some(real_target) = self.module_aliases.get(&name) {
                    if !crate::bridge::module_table::is_native_module(real_target) {
                        IrExprKind::BridgeGet { alias: name }
                    } else {
                        IrExprKind::Var(name)
                    }
                } else {
                    IrExprKind::Var(name)
                }
            }
            _ => self.lower_expr(expr).kind,
        };
        IrExpr { id, kind }
    }

    fn lower_builtin_call(&self, original_id: ExprId, builtin_id: BuiltinId, args: Vec<IrExpr>) -> IrExpr {
        let expected_ty = self.type_table.get(&original_id).cloned();
        let lowered_args: Vec<IrExpr> = args.into_iter().map(|a| self.lower_expr(a)).collect();

        match builtin_id {
            BuiltinId::Sum => {
                if lowered_args.is_empty() {
                    return IrExpr { id: original_id, kind: IrExprKind::IntLit(0) };
                }

                let sum_method = match expected_ty {
                    Some(Type::Float) => "sum::<f64>",
                    Some(Type::Int) => "sum::<i64>",
                    _ => {
                        let arg_ty = lowered_args
                            .get(0)
                            .and_then(|arg| self.type_table.get(&arg.id));
                        match arg_ty {
                            Some(Type::List(inner)) | Some(Type::Set(inner)) => match inner.as_ref() {
                                Type::Float => "sum::<f64>",
                                _ => "sum::<i64>",
                            },
                            _ => "sum::<i64>",
                        }
                    }
                };

                let iter_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(lowered_args[0].clone()),
                        method: "iter".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let sum_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(iter_call),
                        method: sum_method.to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };

                if lowered_args.len() >= 2 {
                    let start = lowered_args[1].clone();
                    let start = match expected_ty {
                        Some(Type::Float) => IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Cast { target: Box::new(start), ty: "f64".to_string() },
                        },
                        Some(Type::Int) => IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Cast { target: Box::new(start), ty: "i64".to_string() },
                        },
                        _ => start,
                    };
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::BinOp {
                            left: Box::new(sum_call),
                            op: IrBinOp::Add,
                            right: Box::new(start),
                        },
                    };
                }
                return IrExpr { id: original_id, kind: sum_call.kind };
            }
            BuiltinId::Any => {
                if lowered_args.is_empty() {
                    return IrExpr { id: original_id, kind: IrExprKind::BoolLit(false) };
                }
                let iter_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(lowered_args[0].clone()),
                        method: "iter".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let any_fn = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::RawCode("|x| *x".to_string()),
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(iter_call),
                        method: "any".to_string(),
                        args: vec![any_fn],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::All => {
                if lowered_args.is_empty() {
                    return IrExpr { id: original_id, kind: IrExprKind::BoolLit(true) };
                }
                let iter_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(lowered_args[0].clone()),
                        method: "iter".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let all_fn = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::RawCode("|x| *x".to_string()),
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(iter_call),
                        method: "all".to_string(),
                        args: vec![all_fn],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Min | BuiltinId::Max => {
                if lowered_args.is_empty() {
                    return IrExpr { id: original_id, kind: IrExprKind::NoneLit };
                }
                let iter_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(lowered_args[0].clone()),
                        method: "iter".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let method = if builtin_id == BuiltinId::Min { "min" } else { "max" };
                let minmax_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(iter_call),
                        method: method.to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let cloned_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(minmax_call),
                        method: "cloned".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(cloned_call),
                        method: "unwrap".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Round => {
                if lowered_args.is_empty() {
                    return IrExpr { id: original_id, kind: IrExprKind::IntLit(0) };
                }

                if lowered_args.len() == 1 {
                    let round_call = IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::MethodCall {
                            target: Box::new(lowered_args[0].clone()),
                            method: "round".to_string(),
                            args: vec![],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                    if matches!(expected_ty, Some(Type::Int)) {
                        return IrExpr {
                            id: original_id,
                            kind: IrExprKind::Cast {
                                target: Box::new(round_call),
                                ty: "i64".to_string(),
                            },
                        };
                    }
                    return IrExpr { id: original_id, kind: round_call.kind };
                }

                if lowered_args.len() >= 2 {
                    let pow_arg = IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::Cast {
                            target: Box::new(lowered_args[1].clone()),
                            ty: "i32".to_string(),
                        },
                    };
                    let factor = IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::MethodCall {
                            target: Box::new(IrExpr { id: self.next_id(), kind: IrExprKind::RawCode("10.0f64".to_string()) }),
                            method: "powi".to_string(),
                            args: vec![pow_arg],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                    let mul = IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::BinOp {
                            left: Box::new(lowered_args[0].clone()),
                            op: IrBinOp::Mul,
                            right: Box::new(factor.clone()),
                        },
                    };
                    let round_call = IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::MethodCall {
                            target: Box::new(mul),
                            method: "round".to_string(),
                            args: vec![],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::BinOp {
                            left: Box::new(round_call),
                            op: IrBinOp::Div,
                            right: Box::new(factor),
                        },
                    };
                }
            }
            BuiltinId::Chr => {
                if lowered_args.is_empty() {
                    return IrExpr { id: original_id, kind: IrExprKind::StringLit(String::new()) };
                }
                let cast_u32 = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Cast { target: Box::new(lowered_args[0].clone()), ty: "u32".to_string() },
                };
                let from_u32 = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Call {
                        func: Box::new(IrExpr { id: self.next_id(), kind: IrExprKind::RawCode("std::char::from_u32".to_string()) }),
                        args: vec![cast_u32],
                        callee_may_raise: false,
                        callee_needs_bridge: false,
                    },
                };
                let unwrap_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(from_u32),
                        method: "unwrap".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(unwrap_call),
                        method: "to_string".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Ord => {
                if lowered_args.is_empty() {
                    return IrExpr { id: original_id, kind: IrExprKind::IntLit(0) };
                }
                let chars_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(lowered_args[0].clone()),
                        method: "chars".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let next_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(chars_call),
                        method: "next".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let unwrap_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(next_call),
                        method: "unwrap".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let cast_u32 = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Cast { target: Box::new(unwrap_call), ty: "u32".to_string() },
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::Cast {
                        target: Box::new(cast_u32),
                        ty: "i64".to_string(),
                    },
                };
            }
            BuiltinId::Bin | BuiltinId::Hex | BuiltinId::Oct => {
                if lowered_args.is_empty() {
                    return IrExpr { id: original_id, kind: IrExprKind::StringLit(String::new()) };
                }
                let fmt = match builtin_id {
                    BuiltinId::Bin => "0b{:b}",
                    BuiltinId::Hex => "0x{:x}",
                    BuiltinId::Oct => "0o{:o}",
                    _ => unreachable!(),
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::Call {
                        func: Box::new(IrExpr { id: self.next_id(), kind: IrExprKind::RawCode("format!".to_string()) }),
                        args: vec![
                            IrExpr { id: self.next_id(), kind: IrExprKind::StringLit(fmt.to_string()) },
                            lowered_args[0].clone(),
                        ],
                        callee_may_raise: false,
                        callee_needs_bridge: false,
                    },
                };
            }
            _ => {}
        }

        // Special-case range() to finalize structure at Lowering (avoid emitting raw range(...) calls).
        if builtin_id == BuiltinId::Range {
            return match lowered_args.len() {
                1 => {
                    // range(end) -> 0..end
                    let start = IrExpr { id: self.next_id(), kind: IrExprKind::IntLit(0) };
                    let end = lowered_args.into_iter().next().unwrap();
                    IrExpr {
                        id: original_id,
                        kind: IrExprKind::Range {
                            start: Box::new(start),
                            end: Box::new(end),
                        },
                    }
                }
                2 => {
                    // range(start, end) -> start..end
                    let mut it = lowered_args.into_iter();
                    let start = it.next().unwrap();
                    let end = it.next().unwrap();
                    IrExpr {
                        id: original_id,
                        kind: IrExprKind::Range {
                            start: Box::new(start),
                            end: Box::new(end),
                        },
                    }
                }
                _ => {
                    // range() or range(start, end, step) remains a builtin for now
                    IrExpr { id: original_id, kind: IrExprKind::BuiltinCall { id: builtin_id, args: lowered_args } }
                }
            };
        }

        let spec = crate::bridge::builtin_table::BUILTIN_SPECS.iter().find(|s| s.id == builtin_id).unwrap();
        
        match spec.kind {
            BuiltinKind::Bridge { target } => {
                let lowered_args: Vec<IrExpr> = lowered_args.into_iter().map(|lowered| {
                    IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::Ref(Box::new(IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::TnkValueFrom(Box::new(lowered)),
                        })),
                    }
                }).collect();

                let bridge_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::BridgeCall {
                        target: Box::new(IrExpr { 
                            id: self.next_id(), 
                            kind: IrExprKind::RawCode(format!("\"{}\"", target))
                        }),
                        args: lowered_args,
                        keywords: vec![],
                    },
                };

                if let Some(expected_ty) = self.type_table.get(&original_id) {
                    if *expected_ty != Type::Any && *expected_ty != Type::Unknown {
                        return IrExpr {
                            id: original_id,
                            kind: IrExprKind::FromTnkValue {
                                value: Box::new(bridge_call),
                                to_type: expected_ty.clone(),
                            },
                        };
                    }
                }
                IrExpr { id: original_id, kind: bridge_call.kind }
            }
            BuiltinKind::NativeMethod { method } => {
                let mut lowered_args = lowered_args.into_iter();
                let receiver = lowered_args.next().expect("NativeMethod requires at least one argument");
                
                IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(receiver),
                        method: method.to_string(),
                        args: lowered_args.collect(),
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                }
            }
            BuiltinKind::Intrinsic { op: _ } => {
                IrExpr { id: original_id, kind: IrExprKind::BuiltinCall { id: builtin_id, args: lowered_args } }
            }
        }
    }
}
