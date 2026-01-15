//! Lowering Pass
//!
//! Semantic Analysis (型推論) の後、Emitter (コード生成) の前に行う
//! IR正規化・最適化パス。

use crate::bridge::builtin_table::BuiltinKind;
use crate::ir::exprs::{BuiltinId, ExprId, IrExpr, IrExprKind};
use crate::ir::nodes::IrNode;
use crate::ir::ops::{IrBinOp, IrUnaryOp};
use crate::semantic::Type;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct LoweringPass {
    module_aliases: HashMap<String, String>,
    type_table: HashMap<ExprId, Type>,
    next_id: Cell<u32>,
    bridge_batch_vars: RefCell<Vec<String>>,
}

impl LoweringPass {
    pub fn new(
        module_aliases: HashMap<String, String>,
        type_table: HashMap<ExprId, Type>,
        next_id_start: u32,
    ) -> Self {
        Self {
            module_aliases,
            type_table,
            next_id: Cell::new(next_id_start),
            bridge_batch_vars: RefCell::new(Vec::new()),
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
                name,
                params,
                body,
                ret,
                hoisted_vars,
                may_raise,
                needs_bridge,
            } => {
                let new_body = body.into_iter().map(|n| self.lower_node(n)).collect();
                IrNode::FuncDecl {
                    name,
                    params,
                    body: new_body,
                    ret,
                    hoisted_vars,
                    may_raise,
                    needs_bridge,
                }
            }
            IrNode::MethodDecl {
                name,
                params,
                body,
                ret,
                takes_self,
                takes_mut_self,
                may_raise,
                needs_bridge,
            } => {
                let new_body = body.into_iter().map(|n| self.lower_node(n)).collect();
                IrNode::MethodDecl {
                    name,
                    params,
                    body: new_body,
                    ret,
                    takes_self,
                    takes_mut_self,
                    may_raise,
                    needs_bridge,
                }
            }
            IrNode::ImplBlock {
                struct_name,
                methods,
            } => {
                let new_methods = methods.into_iter().map(|m| self.lower_node(m)).collect();
                IrNode::ImplBlock {
                    struct_name,
                    methods: new_methods,
                }
            }
            IrNode::Assign { target, value } => IrNode::Assign {
                target,
                value: Box::new(self.lower_expr(*value)),
            },
            IrNode::IndexAssign {
                target,
                index,
                value,
            } => IrNode::IndexAssign {
                target: Box::new(self.lower_expr(*target)),
                index: Box::new(self.lower_expr(*index)),
                value: Box::new(self.lower_expr(*value)),
            },
            IrNode::VarDecl {
                name,
                ty,
                mutable,
                init,
            } => IrNode::VarDecl {
                name,
                ty,
                mutable,
                init: init.map(|e| Box::new(self.lower_expr(*e))),
            },
            IrNode::FieldAssign {
                target,
                field,
                value,
            } => IrNode::FieldAssign {
                target: Box::new(self.lower_expr(*target)),
                field,
                value: Box::new(self.lower_expr(*value)),
            },
            IrNode::AugAssign { target, op, value } => IrNode::AugAssign {
                target,
                op,
                value: Box::new(self.lower_expr(*value)),
            },
            IrNode::MultiAssign { targets, value } => IrNode::MultiAssign {
                targets,
                value: Box::new(self.lower_expr(*value)),
            },
            IrNode::MultiVarDecl { targets, value } => IrNode::MultiVarDecl {
                targets,
                value: Box::new(self.lower_expr(*value)),
            },
            IrNode::Expr(expr) => IrNode::Expr(self.lower_expr(expr)),
            IrNode::Return(Some(expr)) => IrNode::Return(Some(Box::new(self.lower_expr(*expr)))),
            IrNode::If {
                cond,
                then_block,
                else_block,
            } => IrNode::If {
                cond: Box::new(self.lower_expr(*cond)),
                then_block: then_block.into_iter().map(|n| self.lower_node(n)).collect(),
                else_block: else_block
                    .map(|block| block.into_iter().map(|n| self.lower_node(n)).collect()),
            },
            IrNode::While { cond, body } => IrNode::While {
                cond: Box::new(self.lower_expr(*cond)),
                body: body.into_iter().map(|n| self.lower_node(n)).collect(),
            },
            IrNode::For {
                var,
                var_type,
                iter,
                body,
            } => {
                let lowered_iter = self.lower_expr(*iter);
                let use_bridge_batch =
                    !var.contains(',') && self.iter_expr_uses_bridge(&lowered_iter);
                if use_bridge_batch {
                    self.bridge_batch_vars.borrow_mut().push(var.clone());
                    let lowered_body: Vec<IrNode> =
                        body.into_iter().map(|n| self.lower_node(n)).collect();
                    self.bridge_batch_vars.borrow_mut().pop();
                    IrNode::BridgeBatchFor {
                        var,
                        var_type,
                        iter: Box::new(lowered_iter),
                        body: lowered_body,
                    }
                } else {
                    let lowered_body: Vec<IrNode> =
                        body.into_iter().map(|n| self.lower_node(n)).collect();
                    IrNode::For {
                        var,
                        var_type,
                        iter: Box::new(lowered_iter),
                        body: lowered_body,
                    }
                }
            }
            IrNode::TryBlock {
                try_body,
                except_body,
                except_var,
                else_body,
                finally_body,
            } => IrNode::TryBlock {
                try_body: try_body.into_iter().map(|n| self.lower_node(n)).collect(),
                except_body: except_body
                    .into_iter()
                    .map(|n| self.lower_node(n))
                    .collect(),
                except_var,
                else_body: else_body.map(|b| b.into_iter().map(|n| self.lower_node(n)).collect()),
                finally_body: finally_body
                    .map(|b| b.into_iter().map(|n| self.lower_node(n)).collect()),
            },
            IrNode::Assert { test, msg } => IrNode::Assert {
                test: Box::new(self.lower_expr(*test)),
                msg: msg.map(|e| Box::new(self.lower_expr(*e))),
            },
            IrNode::Raise {
                exc_type,
                message,
                cause,
                line,
            } => IrNode::Raise {
                exc_type,
                message: Box::new(self.lower_expr(*message)),
                cause: cause.map(|e| Box::new(self.lower_expr(*e))),
                line,
            },
            IrNode::Sequence(nodes) => {
                IrNode::Sequence(nodes.into_iter().map(|n| self.lower_node(n)).collect())
            }
            IrNode::Block { stmts } => IrNode::Block {
                stmts: stmts.into_iter().map(|n| self.lower_node(n)).collect(),
            },
            IrNode::Match { value, arms } => IrNode::Match {
                value: self.lower_expr(value),
                arms: arms
                    .into_iter()
                    .map(|arm| crate::ir::nodes::MatchArm {
                        variant: arm.variant,
                        binding: arm.binding,
                        body: arm.body.into_iter().map(|n| self.lower_node(n)).collect(),
                    })
                    .collect(),
            },
            _ => node,
        }
    }

    fn iter_expr_uses_bridge(&self, expr: &IrExpr) -> bool {
        match &expr.kind {
            IrExprKind::BridgeMethodCall { .. }
            | IrExprKind::BridgeCall { .. }
            | IrExprKind::BridgeAttributeAccess { .. }
            | IrExprKind::BridgeItemAccess { .. }
            | IrExprKind::BridgeSlice { .. }
            | IrExprKind::BridgeGet { .. } => true,
            IrExprKind::Ref(inner) | IrExprKind::TnkValueFrom(inner) => {
                self.iter_expr_uses_bridge(inner)
            }
            _ => false,
        }
    }

    fn is_bridge_batch_var(&self, name: &str) -> bool {
        self.bridge_batch_vars.borrow().iter().any(|v| v == name)
    }

    fn lower_expr(&self, expr: IrExpr) -> IrExpr {
        let id = expr.id;
        let kind = match expr.kind {
            IrExprKind::BuiltinCall {
                id: builtin_id,
                args,
            } => {
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
            IrExprKind::MethodCall {
                target,
                method,
                args,
                target_type,
                callee_needs_bridge,
            } => IrExprKind::MethodCall {
                target: Box::new(self.lower_expr_as_target(*target)),
                method,
                args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                target_type,
                callee_needs_bridge,
            },
            IrExprKind::BridgeMethodCall {
                target,
                method,
                args,
                keywords,
            } => IrExprKind::BridgeMethodCall {
                target: Box::new(self.lower_expr_as_target(*target)),
                method,
                args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                keywords: keywords
                    .into_iter()
                    .map(|(k, v)| (k, self.lower_expr(v)))
                    .collect(),
            },
            IrExprKind::BridgeAttributeAccess { target, attribute } => {
                IrExprKind::BridgeAttributeAccess {
                    target: Box::new(self.lower_expr_as_target(*target)),
                    attribute,
                }
            }
            IrExprKind::BridgeItemAccess { target, index } => IrExprKind::BridgeItemAccess {
                target: Box::new(self.lower_expr_as_target(*target)),
                index: Box::new(self.lower_expr(*index)),
            },
            IrExprKind::BridgeSlice {
                target,
                start,
                stop,
                step,
            } => IrExprKind::BridgeSlice {
                target: Box::new(self.lower_expr_as_target(*target)),
                start: Box::new(self.lower_expr(*start)),
                stop: Box::new(self.lower_expr(*stop)),
                step: Box::new(self.lower_expr(*step)),
            },
            IrExprKind::PyO3MethodCall {
                target,
                method,
                args,
            } => IrExprKind::PyO3MethodCall {
                target: Box::new(self.lower_expr_as_target(*target)),
                method,
                args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
            },
            IrExprKind::PyO3Call {
                module,
                method,
                args,
            } => IrExprKind::PyO3Call {
                module,
                method,
                args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
            },
            IrExprKind::TnkValueFrom(inner) => {
                IrExprKind::TnkValueFrom(Box::new(self.lower_expr_as_target(*inner)))
            }
            IrExprKind::BinOp { left, op, right } => IrExprKind::BinOp {
                left: Box::new(self.lower_expr(*left)),
                op,
                right: Box::new(self.lower_expr(*right)),
            },
            IrExprKind::Call {
                func,
                args,
                callee_may_raise,
                callee_needs_bridge,
            } => {
                let lowered_func = self.lower_expr_as_target(*func);
                if matches!(lowered_func.kind, IrExprKind::BridgeGet { .. }) {
                    let bridge_call = IrExpr {
                        id,
                        kind: IrExprKind::BridgeCall {
                            target: Box::new(lowered_func),
                            args: args
                                .into_iter()
                                .map(|a| {
                                    let lowered_arg = self.lower_expr(a);
                                    IrExpr {
                                        id: self.next_id(),
                                        kind: IrExprKind::Ref(Box::new(IrExpr {
                                            id: self.next_id(),
                                            kind: IrExprKind::TnkValueFrom(Box::new(lowered_arg)),
                                        })),
                                    }
                                })
                                .collect(),
                            keywords: vec![],
                        },
                    };
                    return self.wrap_bridge_result(bridge_call);
                }
                IrExprKind::Call {
                    func: Box::new(lowered_func),
                    args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                    callee_may_raise,
                    callee_needs_bridge,
                }
            }
            IrExprKind::StaticCall { path, args } => IrExprKind::StaticCall {
                path,
                args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
            },
            IrExprKind::FieldAccess { target, field } => IrExprKind::FieldAccess {
                target: Box::new(self.lower_expr(*target)),
                field,
            },
            IrExprKind::UnaryOp { op, operand } => IrExprKind::UnaryOp {
                op,
                operand: Box::new(self.lower_expr(*operand)),
            },
            IrExprKind::List {
                elem_type,
                elements,
            } => IrExprKind::List {
                elem_type,
                elements: elements.into_iter().map(|e| self.lower_expr(e)).collect(),
            },
            IrExprKind::Set {
                elem_type,
                elements,
            } => IrExprKind::Set {
                elem_type,
                elements: elements.into_iter().map(|e| self.lower_expr(e)).collect(),
            },
            IrExprKind::ListComp {
                elt,
                target,
                iter,
                condition,
            } => IrExprKind::ListComp {
                elt: Box::new(self.lower_expr(*elt)),
                target,
                iter: Box::new(self.lower_expr(*iter)),
                condition: condition.map(|c| Box::new(self.lower_expr(*c))),
            },
            IrExprKind::SetComp {
                elt,
                target,
                iter,
                condition,
            } => IrExprKind::SetComp {
                elt: Box::new(self.lower_expr(*elt)),
                target,
                iter: Box::new(self.lower_expr(*iter)),
                condition: condition.map(|c| Box::new(self.lower_expr(*c))),
            },
            IrExprKind::DictComp {
                key,
                value,
                target,
                iter,
                condition,
            } => IrExprKind::DictComp {
                key: Box::new(self.lower_expr(*key)),
                value: Box::new(self.lower_expr(*value)),
                target,
                iter: Box::new(self.lower_expr(*iter)),
                condition: condition.map(|c| Box::new(self.lower_expr(*c))),
            },
            IrExprKind::Ref(inner) => IrExprKind::Ref(Box::new(self.lower_expr(*inner))),
            IrExprKind::Dict {
                key_type,
                value_type,
                entries,
            } => IrExprKind::Dict {
                key_type,
                value_type,
                entries: entries
                    .into_iter()
                    .map(|(k, v)| (self.lower_expr(k), self.lower_expr(v)))
                    .collect(),
            },
            IrExprKind::Tuple(elements) => {
                IrExprKind::Tuple(elements.into_iter().map(|e| self.lower_expr(e)).collect())
            }
            IrExprKind::Index { target, index } => IrExprKind::Index {
                target: Box::new(self.lower_expr(*target)),
                index: Box::new(self.lower_expr(*index)),
            },
            IrExprKind::Slice {
                target,
                start,
                end,
                step,
            } => IrExprKind::Slice {
                target: Box::new(self.lower_expr(*target)),
                start: start.map(|s| Box::new(self.lower_expr(*s))),
                end: end.map(|e| Box::new(self.lower_expr(*e))),
                step: step.map(|s| Box::new(self.lower_expr(*s))),
            },
            IrExprKind::FString { parts, values } => IrExprKind::FString {
                parts,
                values: values
                    .into_iter()
                    .map(|(v, ty)| (self.lower_expr(v), ty))
                    .collect(),
            },
            IrExprKind::Print { args } => IrExprKind::Print {
                args: args
                    .into_iter()
                    .map(|(v, ty)| (self.lower_expr(v), ty))
                    .collect(),
            },
            IrExprKind::IfExp { test, body, orelse } => IrExprKind::IfExp {
                test: Box::new(self.lower_expr(*test)),
                body: Box::new(self.lower_expr(*body)),
                orelse: Box::new(self.lower_expr(*orelse)),
            },
            IrExprKind::Closure {
                params,
                body,
                ret_type,
            } => IrExprKind::Closure {
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
            IrExprKind::ConstRef { path } => IrExprKind::ConstRef { path },
            IrExprKind::JsonConversion { target, convert_to } => IrExprKind::JsonConversion {
                target: Box::new(self.lower_expr(*target)),
                convert_to,
            },
            IrExprKind::BridgeGet { alias } => IrExprKind::TnkValueFrom(Box::new(IrExpr {
                id: self.next_id(),
                kind: IrExprKind::BridgeGet { alias },
            })),
            IrExprKind::BridgeCall {
                target,
                args,
                keywords,
            } => IrExprKind::BridgeCall {
                target: Box::new(self.lower_expr_as_target(*target)),
                args: args.into_iter().map(|a| self.lower_expr(a)).collect(),
                keywords: keywords
                    .into_iter()
                    .map(|(k, v)| (k, self.lower_expr(v)))
                    .collect(),
            },
            _ => expr.kind,
        };
        let expr = IrExpr { id, kind };
        self.wrap_bridge_result(expr)
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

    fn wrap_bridge_result(&self, expr: IrExpr) -> IrExpr {
        if matches!(expr.kind, IrExprKind::FromTnkValue { .. }) {
            return expr;
        }
        let expected_ty = match self.type_table.get(&expr.id) {
            Some(t) => t,
            None => return expr,
        };
        if matches!(expected_ty, Type::Any | Type::Unknown) {
            return expr;
        }
        let is_bridge_result = matches!(
            expr.kind,
            IrExprKind::BridgeCall { .. }
                | IrExprKind::BridgeMethodCall { .. }
                | IrExprKind::BridgeAttributeAccess { .. }
                | IrExprKind::BridgeItemAccess { .. }
                | IrExprKind::BridgeSlice { .. }
        );
        if !is_bridge_result {
            return expr;
        }
        IrExpr {
            id: expr.id,
            kind: IrExprKind::FromTnkValue {
                value: Box::new(expr),
                to_type: expected_ty.clone(),
            },
        }
    }

    #[allow(dead_code)]
    fn format_simple_expr(&self, expr: &IrExpr) -> String {
        match &expr.kind {
            IrExprKind::IntLit(n) => format!("{n}i64"),
            IrExprKind::FloatLit(f) => format!("{f}"),
            IrExprKind::BoolLit(b) => b.to_string(),
            IrExprKind::StringLit(s) => format!("\"{s}\""),
            IrExprKind::Var(name) => name.clone(),
            IrExprKind::Cast { target, ty } => {
                format!("({} as {})", self.format_simple_expr(target), ty)
            }
            IrExprKind::UnaryOp { op, operand } => match op {
                crate::ir::ops::IrUnaryOp::Neg => format!("-{}", self.format_simple_expr(operand)),
                _ => self.format_simple_expr(operand),
            },
            IrExprKind::BinOp { left, op, right } => {
                let op_str = match op {
                    IrBinOp::Add => "+",
                    IrBinOp::Sub => "-",
                    IrBinOp::Mul => "*",
                    IrBinOp::Div => "/",
                    IrBinOp::Mod => "%",
                    IrBinOp::Eq => "==",
                    IrBinOp::NotEq => "!=",
                    IrBinOp::Lt => "<",
                    IrBinOp::LtEq => "<=",
                    IrBinOp::Gt => ">",
                    IrBinOp::GtEq => ">=",
                    IrBinOp::And => "&&",
                    IrBinOp::Or => "||",
                    _ => "+",
                };
                format!(
                    "{} {} {}",
                    self.format_simple_expr(left),
                    op_str,
                    self.format_simple_expr(right)
                )
            }
            _ => "0".to_string(),
        }
    }

    fn lower_builtin_call(
        &self,
        original_id: ExprId,
        builtin_id: BuiltinId,
        args: Vec<IrExpr>,
    ) -> IrExpr {
        let expected_ty = self.type_table.get(&original_id).cloned();
        let lowered_args: Vec<IrExpr> = args.into_iter().map(|a| self.lower_expr(a)).collect();

        match builtin_id {
            BuiltinId::Sum => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::IntLit(0),
                    };
                }

                let sum_method = match expected_ty {
                    Some(Type::Float) => "sum::<f64>",
                    Some(Type::Int) => "sum::<i64>",
                    _ => {
                        let arg_ty = lowered_args
                            .first()
                            .and_then(|arg| self.type_table.get(&arg.id));
                        match arg_ty {
                            Some(Type::List(inner)) | Some(Type::Set(inner)) => {
                                match inner.as_ref() {
                                    Type::Float => "sum::<f64>",
                                    _ => "sum::<i64>",
                                }
                            }
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
                            kind: IrExprKind::Cast {
                                target: Box::new(start),
                                ty: "f64".to_string(),
                            },
                        },
                        Some(Type::Int) => IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Cast {
                                target: Box::new(start),
                                ty: "i64".to_string(),
                            },
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
                return IrExpr {
                    id: original_id,
                    kind: sum_call.kind,
                };
            }
            BuiltinId::Int => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::IntLit(0),
                    };
                }
                if let IrExprKind::Var(name) = &lowered_args[0].kind {
                    if self.is_bridge_batch_var(name) {
                        return IrExpr {
                            id: original_id,
                            kind: IrExprKind::JsonConversion {
                                target: Box::new(lowered_args[0].clone()),
                                convert_to: "i64".to_string(),
                            },
                        };
                    }
                }
                let arg_ty = lowered_args
                    .first()
                    .and_then(|arg| self.type_table.get(&arg.id));
                if matches!(arg_ty, Some(Type::Any | Type::Unknown)) {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::JsonConversion {
                            target: Box::new(lowered_args[0].clone()),
                            convert_to: "i64".to_string(),
                        },
                    };
                }
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::Cast {
                        target: Box::new(lowered_args[0].clone()),
                        ty: "i64".to_string(),
                    },
                };
            }
            BuiltinId::Float => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::FloatLit(0.0),
                    };
                }
                if let IrExprKind::Var(name) = &lowered_args[0].kind {
                    if self.is_bridge_batch_var(name) {
                        return IrExpr {
                            id: original_id,
                            kind: IrExprKind::JsonConversion {
                                target: Box::new(lowered_args[0].clone()),
                                convert_to: "f64".to_string(),
                            },
                        };
                    }
                }
                let arg_ty = lowered_args
                    .first()
                    .and_then(|arg| self.type_table.get(&arg.id));
                if matches!(arg_ty, Some(Type::Any | Type::Unknown)) {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::JsonConversion {
                            target: Box::new(lowered_args[0].clone()),
                            convert_to: "f64".to_string(),
                        },
                    };
                }
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::Cast {
                        target: Box::new(lowered_args[0].clone()),
                        ty: "f64".to_string(),
                    },
                };
            }
            BuiltinId::Str => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::StringLit(String::new()),
                    };
                }
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(lowered_args[0].clone()),
                        method: "to_string".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Print => {
                let typed_args: Vec<(IrExpr, Type)> = lowered_args
                    .into_iter()
                    .map(|arg| {
                        let ty = self
                            .type_table
                            .get(&arg.id)
                            .cloned()
                            .unwrap_or(Type::Unknown);
                        (arg, ty)
                    })
                    .collect();
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::Print { args: typed_args },
                };
            }
            BuiltinId::Sorted => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::List {
                            elem_type: Type::Unknown,
                            elements: vec![],
                        },
                    };
                }
                let iter = lowered_args[0].clone();
                let key = if lowered_args.len() >= 2 {
                    match &lowered_args[1].kind {
                        IrExprKind::NoneLit => None,
                        IrExprKind::BoxNew(inner) => Some(Box::new((**inner).clone())),
                        _ => Some(Box::new(lowered_args[1].clone())),
                    }
                } else {
                    None
                };
                let reverse = if lowered_args.len() >= 3 {
                    matches!(lowered_args[2].kind, IrExprKind::BoolLit(true))
                } else {
                    false
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::Sorted {
                        iter: Box::new(iter),
                        key,
                        reverse,
                    },
                };
            }
            BuiltinId::Set => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::Set {
                            elem_type: Type::Unknown,
                            elements: vec![],
                        },
                    };
                }
                let arg = lowered_args[0].clone();
                let iter_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(arg),
                        method: "iter".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let cloned_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(iter_call),
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
                        method: "collect_hashset".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::List => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::List {
                            elem_type: Type::Unknown,
                            elements: vec![],
                        },
                    };
                }
                let arg = lowered_args[0].clone();
                if let IrExprKind::Call {
                    func,
                    args: call_args,
                    ..
                } = &arg.kind
                {
                    if let IrExprKind::Var(func_name) = &func.kind {
                        if func_name == "map" && call_args.len() == 2 {
                            let mut lambda = call_args[0].clone();
                            if let IrExprKind::BoxNew(inner) = lambda.kind {
                                lambda = *inner;
                            }
                            let iterable = call_args[1].clone();
                            let iter_call = IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::MethodCall {
                                    target: Box::new(iterable),
                                    method: "iter".to_string(),
                                    args: vec![],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            };
                            let cloned_call = IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::MethodCall {
                                    target: Box::new(iter_call),
                                    method: "cloned".to_string(),
                                    args: vec![],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            };
                            let map_call = IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::MethodCall {
                                    target: Box::new(cloned_call),
                                    method: "map".to_string(),
                                    args: vec![lambda],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            };
                            return IrExpr {
                                id: original_id,
                                kind: IrExprKind::MethodCall {
                                    target: Box::new(map_call),
                                    method: "collect".to_string(),
                                    args: vec![],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            };
                        }
                        if func_name == "filter" && call_args.len() == 2 {
                            let mut lambda = call_args[0].clone();
                            if let IrExprKind::BoxNew(inner) = lambda.kind {
                                lambda = *inner;
                            }
                            let filter_closure = if let IrExprKind::Closure {
                                params,
                                body,
                                ret_type,
                            } = &lambda.kind
                            {
                                if params.len() == 1 {
                                    let param = &params[0];
                                    if !param.starts_with('&') && !param.contains('(') {
                                        IrExpr {
                                            id: lambda.id,
                                            kind: IrExprKind::Closure {
                                                params: vec![format!("&{param}")],
                                                body: body.clone(),
                                                ret_type: ret_type.clone(),
                                            },
                                        }
                                    } else {
                                        lambda.clone()
                                    }
                                } else {
                                    lambda.clone()
                                }
                            } else {
                                lambda.clone()
                            };
                            let iterable = call_args[1].clone();
                            let iter_call = IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::MethodCall {
                                    target: Box::new(iterable),
                                    method: "iter".to_string(),
                                    args: vec![],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            };
                            let cloned_call = IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::MethodCall {
                                    target: Box::new(iter_call),
                                    method: "cloned".to_string(),
                                    args: vec![],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            };
                            let filter_call = IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::MethodCall {
                                    target: Box::new(cloned_call),
                                    method: "filter".to_string(),
                                    args: vec![filter_closure],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            };
                            return IrExpr {
                                id: original_id,
                                kind: IrExprKind::MethodCall {
                                    target: Box::new(filter_call),
                                    method: "collect".to_string(),
                                    args: vec![],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            };
                        }
                    }
                }
                if let IrExprKind::MethodCall {
                    target,
                    method,
                    target_type,
                    ..
                } = &arg.kind
                {
                    let target_ty = self.type_table.get(&target.id);
                    let is_dict = matches!(target_ty, Some(Type::Dict(_, _)))
                        || matches!(target_type, Type::Dict(_, _));
                    if is_dict && (method == "items" || method == "iter") {
                        let iter_call = if method == "iter" {
                            arg.clone()
                        } else {
                            IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::MethodCall {
                                    target: Box::new((**target).clone()),
                                    method: "iter".to_string(),
                                    args: vec![],
                                    target_type: Type::Unknown,
                                    callee_needs_bridge: false,
                                },
                            }
                        };
                        let k_var = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Var("k".to_string()),
                        };
                        let v_var = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Var("v".to_string()),
                        };
                        let deref_k = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::UnaryOp {
                                op: IrUnaryOp::Deref,
                                operand: Box::new(k_var),
                            },
                        };
                        let clone_v = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::MethodCall {
                                target: Box::new(v_var),
                                method: "clone".to_string(),
                                args: vec![],
                                target_type: Type::Unknown,
                                callee_needs_bridge: false,
                            },
                        };
                        let tuple_expr = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Tuple(vec![deref_k, clone_v]),
                        };
                        let map_fn = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Closure {
                                params: vec!["(k, v)".to_string()],
                                body: vec![IrNode::Return(Some(Box::new(tuple_expr)))],
                                ret_type: Type::Unknown,
                            },
                        };
                        let map_call = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::MethodCall {
                                target: Box::new(iter_call),
                                method: "map".to_string(),
                                args: vec![map_fn],
                                target_type: Type::Unknown,
                                callee_needs_bridge: false,
                            },
                        };
                        return IrExpr {
                            id: original_id,
                            kind: IrExprKind::MethodCall {
                                target: Box::new(map_call),
                                method: "collect::<Vec<_>>".to_string(),
                                args: vec![],
                                target_type: Type::Unknown,
                                callee_needs_bridge: false,
                            },
                        };
                    }
                }
                if matches!(arg.kind, IrExprKind::Range { .. }) {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::MethodCall {
                            target: Box::new(arg),
                            method: "collect::<Vec<_>>".to_string(),
                            args: vec![],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                }
                if matches!(
                    arg.kind,
                    IrExprKind::MethodCall { .. } | IrExprKind::ListComp { .. }
                ) {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::MethodCall {
                            target: Box::new(arg),
                            method: "collect::<Vec<_>>".to_string(),
                            args: vec![],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                }
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(arg),
                        method: "to_vec".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Tuple => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::Tuple(vec![]),
                    };
                }
                let arg = lowered_args[0].clone();
                if matches!(
                    arg.kind,
                    IrExprKind::ListComp { .. } | IrExprKind::MethodCall { .. }
                ) {
                    return IrExpr {
                        id: original_id,
                        kind: arg.kind,
                    };
                }
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(arg),
                        method: "collect::<Vec<_>>".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Dict => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::Dict {
                            key_type: Type::Unknown,
                            value_type: Type::Unknown,
                            entries: vec![],
                        },
                    };
                }
                let arg = lowered_args[0].clone();
                let arg_ty = self.type_table.get(&arg.id);
                if matches!(arg_ty, Some(Type::Dict(_, _))) {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::MethodCall {
                            target: Box::new(arg),
                            method: "clone".to_string(),
                            args: vec![],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                }
                if matches!(
                    arg.kind,
                    IrExprKind::MethodCall { .. }
                        | IrExprKind::ListComp { .. }
                        | IrExprKind::DictComp { .. }
                ) {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::MethodCall {
                            target: Box::new(arg),
                            method: "collect::<std::collections::HashMap<_, _>>".to_string(),
                            args: vec![],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                }
                let iter_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(arg),
                        method: "iter".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let k_var = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Var("k".to_string()),
                };
                let v_var = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Var("v".to_string()),
                };
                let clone_k = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(k_var),
                        method: "clone".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let clone_v = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(v_var),
                        method: "clone".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let tuple_expr = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Tuple(vec![clone_k, clone_v]),
                };
                let map_fn = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Closure {
                        params: vec!["(k, v)".to_string()],
                        body: vec![IrNode::Expr(tuple_expr)],
                        ret_type: Type::Unknown,
                    },
                };
                let map_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(iter_call),
                        method: "map".to_string(),
                        args: vec![map_fn],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(map_call),
                        method: "collect::<std::collections::HashMap<_, _>>".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Enumerate => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::BuiltinCall {
                            id: builtin_id,
                            args: lowered_args,
                        },
                    };
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
                let enum_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(iter_call),
                        method: "enumerate".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let i_var = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Var("i".to_string()),
                };
                let x_var = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Var("x".to_string()),
                };
                let cast_i = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Cast {
                        target: Box::new(i_var),
                        ty: "i64".to_string(),
                    },
                };
                let add_i = if lowered_args.len() >= 2 {
                    IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::BinOp {
                            left: Box::new(cast_i),
                            op: IrBinOp::Add,
                            right: Box::new(lowered_args[1].clone()),
                        },
                    }
                } else {
                    cast_i
                };
                let clone_x = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(x_var),
                        method: "clone".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                let tuple_expr = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Tuple(vec![add_i, clone_x]),
                };
                let map_body = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Closure {
                        params: vec!["(i, x)".to_string()],
                        body: vec![IrNode::Expr(tuple_expr)],
                        ret_type: Type::Unknown,
                    },
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(enum_call),
                        method: "map".to_string(),
                        args: vec![map_body],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Zip => {
                if lowered_args.len() < 2 {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::List {
                            elem_type: Type::Unknown,
                            elements: vec![],
                        },
                    };
                }
                let mut target = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::MethodCall {
                        target: Box::new(lowered_args[0].clone()),
                        method: "iter".to_string(),
                        args: vec![],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
                for arg in lowered_args.iter().skip(1) {
                    let iter_arg = IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::MethodCall {
                            target: Box::new(arg.clone()),
                            method: "iter".to_string(),
                            args: vec![],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                    target = IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::MethodCall {
                            target: Box::new(target),
                            method: "zip".to_string(),
                            args: vec![iter_arg],
                            target_type: Type::Unknown,
                            callee_needs_bridge: false,
                        },
                    };
                }
                let map_body = match lowered_args.len() {
                    2 => {
                        let x_var = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Var("x".to_string()),
                        };
                        let y_var = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Var("y".to_string()),
                        };
                        let clone_x = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::MethodCall {
                                target: Box::new(x_var),
                                method: "clone".to_string(),
                                args: vec![],
                                target_type: Type::Unknown,
                                callee_needs_bridge: false,
                            },
                        };
                        let clone_y = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::MethodCall {
                                target: Box::new(y_var),
                                method: "clone".to_string(),
                                args: vec![],
                                target_type: Type::Unknown,
                                callee_needs_bridge: false,
                            },
                        };
                        let tuple_expr = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Tuple(vec![clone_x, clone_y]),
                        };
                        IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Closure {
                                params: vec!["(x, y)".to_string()],
                                body: vec![IrNode::Expr(tuple_expr)],
                                ret_type: Type::Unknown,
                            },
                        }
                    }
                    3 => {
                        let x_var = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Var("x".to_string()),
                        };
                        let y_var = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Var("y".to_string()),
                        };
                        let z_var = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Var("z".to_string()),
                        };
                        let clone_x = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::MethodCall {
                                target: Box::new(x_var),
                                method: "clone".to_string(),
                                args: vec![],
                                target_type: Type::Unknown,
                                callee_needs_bridge: false,
                            },
                        };
                        let clone_y = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::MethodCall {
                                target: Box::new(y_var),
                                method: "clone".to_string(),
                                args: vec![],
                                target_type: Type::Unknown,
                                callee_needs_bridge: false,
                            },
                        };
                        let clone_z = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::MethodCall {
                                target: Box::new(z_var),
                                method: "clone".to_string(),
                                args: vec![],
                                target_type: Type::Unknown,
                                callee_needs_bridge: false,
                            },
                        };
                        let tuple_expr = IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Tuple(vec![clone_x, clone_y, clone_z]),
                        };
                        IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::Closure {
                                params: vec!["((x, y), z)".to_string()],
                                body: vec![IrNode::Expr(tuple_expr)],
                                ret_type: Type::Unknown,
                            },
                        }
                    }
                    _ => IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::Closure {
                            params: vec!["t".to_string()],
                            body: vec![IrNode::Expr(IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::Var("t".to_string()),
                            })],
                            ret_type: Type::Unknown,
                        },
                    },
                };
                return IrExpr {
                    id: original_id,
                    kind: IrExprKind::MethodCall {
                        target: Box::new(target),
                        method: "map".to_string(),
                        args: vec![map_body],
                        target_type: Type::Unknown,
                        callee_needs_bridge: false,
                    },
                };
            }
            BuiltinId::Any => {
                if lowered_args.is_empty() {
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::BoolLit(false),
                    };
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
                let x_var = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Var("x".to_string()),
                };
                let deref_x = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::UnaryOp {
                        op: IrUnaryOp::Deref,
                        operand: Box::new(x_var),
                    },
                };
                let any_fn = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Closure {
                        params: vec!["x".to_string()],
                        body: vec![IrNode::Expr(deref_x)],
                        ret_type: Type::Unknown,
                    },
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
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::BoolLit(true),
                    };
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
                let x_var = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Var("x".to_string()),
                };
                let deref_x = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::UnaryOp {
                        op: IrUnaryOp::Deref,
                        operand: Box::new(x_var),
                    },
                };
                let all_fn = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Closure {
                        params: vec!["x".to_string()],
                        body: vec![IrNode::Expr(deref_x)],
                        ret_type: Type::Unknown,
                    },
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
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::NoneLit,
                    };
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
                let method = if builtin_id == BuiltinId::Min {
                    "min"
                } else {
                    "max"
                };
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
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::IntLit(0),
                    };
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
                    return IrExpr {
                        id: original_id,
                        kind: round_call.kind,
                    };
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
                            target: Box::new(IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::RawCode("10.0f64".to_string()),
                            }),
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
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::StringLit(String::new()),
                    };
                }
                let cast_u32 = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Cast {
                        target: Box::new(lowered_args[0].clone()),
                        ty: "u32".to_string(),
                    },
                };
                let from_u32 = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::Call {
                        func: Box::new(IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::RawCode("std::char::from_u32".to_string()),
                        }),
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
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::IntLit(0),
                    };
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
                    kind: IrExprKind::Cast {
                        target: Box::new(unwrap_call),
                        ty: "u32".to_string(),
                    },
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
                    return IrExpr {
                        id: original_id,
                        kind: IrExprKind::StringLit(String::new()),
                    };
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
                        func: Box::new(IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::RawCode("format!".to_string()),
                        }),
                        args: vec![
                            IrExpr {
                                id: self.next_id(),
                                kind: IrExprKind::StringLit(fmt.to_string()),
                            },
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
                    let start = IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::IntLit(0),
                    };
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
                    IrExpr {
                        id: original_id,
                        kind: IrExprKind::BuiltinCall {
                            id: builtin_id,
                            args: lowered_args,
                        },
                    }
                }
            };
        }

        let spec = crate::bridge::builtin_table::BUILTIN_SPECS
            .iter()
            .find(|s| s.id == builtin_id)
            .unwrap();

        match spec.kind {
            BuiltinKind::Bridge { target } => {
                let lowered_args: Vec<IrExpr> = lowered_args
                    .into_iter()
                    .map(|lowered| IrExpr {
                        id: self.next_id(),
                        kind: IrExprKind::Ref(Box::new(IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::TnkValueFrom(Box::new(lowered)),
                        })),
                    })
                    .collect();

                let bridge_call = IrExpr {
                    id: self.next_id(),
                    kind: IrExprKind::BridgeCall {
                        target: Box::new(IrExpr {
                            id: self.next_id(),
                            kind: IrExprKind::RawCode(format!("\"{}\"", target)),
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
                IrExpr {
                    id: original_id,
                    kind: bridge_call.kind,
                }
            }
            BuiltinKind::NativeMethod { method } => {
                let mut lowered_args = lowered_args.into_iter();
                let receiver = lowered_args
                    .next()
                    .expect("NativeMethod requires at least one argument");

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
            BuiltinKind::Intrinsic { op: _ } => IrExpr {
                id: original_id,
                kind: IrExprKind::BuiltinCall {
                    id: builtin_id,
                    args: lowered_args,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lower_sequence_dict_builtin_clone() {
        let mut type_table = HashMap::new();
        let arg_id = ExprId(0);
        let call_id = ExprId(1);
        type_table.insert(
            arg_id,
            Type::Dict(Box::new(Type::Int), Box::new(Type::String)),
        );

        let lowering = LoweringPass::new(HashMap::new(), type_table, 100);
        let node = IrNode::Sequence(vec![IrNode::VarDecl {
            name: "x".to_string(),
            ty: Type::Dict(Box::new(Type::Int), Box::new(Type::String)),
            mutable: false,
            init: Some(Box::new(IrExpr {
                id: call_id,
                kind: IrExprKind::BuiltinCall {
                    id: BuiltinId::Dict,
                    args: vec![IrExpr {
                        id: arg_id,
                        kind: IrExprKind::Var("src".to_string()),
                    }],
                },
            })),
        }]);

        let lowered = lowering.apply(vec![node]);
        match &lowered[0] {
            IrNode::Sequence(nodes) => match &nodes[0] {
                IrNode::VarDecl {
                    init: Some(expr), ..
                } => match &expr.kind {
                    IrExprKind::MethodCall { method, .. } => assert_eq!(method, "clone"),
                    _ => panic!("Expected MethodCall clone"),
                },
                _ => panic!("Expected VarDecl in Sequence"),
            },
            _ => panic!("Expected Sequence"),
        }
    }

    #[test]
    fn test_lower_dict_builtin_collect_from_iter() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 200);
        let call_id = ExprId(2);
        let arg_id = ExprId(3);
        let expr = IrExpr {
            id: call_id,
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Dict,
                args: vec![IrExpr {
                    id: arg_id,
                    kind: IrExprKind::Var("pairs".to_string()),
                }],
            },
        };
        let lowered = lowering.lower_expr(expr);
        match lowered.kind {
            IrExprKind::MethodCall { method, target, .. } => {
                assert_eq!(method, "collect::<std::collections::HashMap<_, _>>");
                match target.kind {
                    IrExprKind::MethodCall {
                        method: inner_method,
                        ..
                    } => {
                        assert_eq!(inner_method, "map");
                    }
                    _ => panic!("Expected map() call before collect"),
                }
            }
            _ => panic!("Expected collect MethodCall"),
        }
    }

    #[test]
    fn test_wrap_bridge_result_inserts_from_tnkvalue() {
        let mut type_table = HashMap::new();
        let expr_id = ExprId(10);
        type_table.insert(expr_id, Type::Int);
        let lowering = LoweringPass::new(HashMap::new(), type_table, 10);
        let expr = IrExpr {
            id: expr_id,
            kind: IrExprKind::BridgeCall {
                target: Box::new(IrExpr {
                    id: ExprId(11),
                    kind: IrExprKind::Var("f".to_string()),
                }),
                args: vec![],
                keywords: vec![],
            },
        };
        let wrapped = lowering.wrap_bridge_result(expr);
        match wrapped.kind {
            IrExprKind::FromTnkValue { to_type, .. } => assert_eq!(to_type, Type::Int),
            _ => panic!("Expected FromTnkValue"),
        }
    }

    #[test]
    fn test_wrap_bridge_result_skips_any() {
        let mut type_table = HashMap::new();
        let expr_id = ExprId(12);
        type_table.insert(expr_id, Type::Any);
        let lowering = LoweringPass::new(HashMap::new(), type_table, 12);
        let expr = IrExpr {
            id: expr_id,
            kind: IrExprKind::BridgeCall {
                target: Box::new(IrExpr {
                    id: ExprId(13),
                    kind: IrExprKind::Var("f".to_string()),
                }),
                args: vec![],
                keywords: vec![],
            },
        };
        let wrapped = lowering.wrap_bridge_result(expr);
        assert!(matches!(wrapped.kind, IrExprKind::BridgeCall { .. }));
    }

    #[test]
    fn test_lower_enumerate_with_start() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 300);
        let expr = IrExpr {
            id: ExprId(20),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Enumerate,
                args: vec![
                    IrExpr {
                        id: ExprId(21),
                        kind: IrExprKind::Var("items".to_string()),
                    },
                    IrExpr {
                        id: ExprId(22),
                        kind: IrExprKind::IntLit(5),
                    },
                ],
            },
        };
        let lowered = lowering.lower_expr(expr);
        match lowered.kind {
            IrExprKind::MethodCall { method, args, .. } => {
                assert_eq!(method, "map");
                assert!(matches!(args[0].kind, IrExprKind::Closure { .. }));
            }
            _ => panic!("Expected enumerate map call"),
        }
    }

    #[test]
    fn test_lower_zip_two_args() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 400);
        let expr = IrExpr {
            id: ExprId(30),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Zip,
                args: vec![
                    IrExpr {
                        id: ExprId(31),
                        kind: IrExprKind::Var("a".to_string()),
                    },
                    IrExpr {
                        id: ExprId(32),
                        kind: IrExprKind::Var("b".to_string()),
                    },
                ],
            },
        };
        let lowered = lowering.lower_expr(expr);
        match lowered.kind {
            IrExprKind::MethodCall { method, args, .. } => {
                assert_eq!(method, "map");
                assert!(matches!(args[0].kind, IrExprKind::Closure { .. }));
            }
            _ => panic!("Expected zip map call"),
        }
    }

    #[test]
    fn test_lower_range_one_arg() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 500);
        let expr = IrExpr {
            id: ExprId(40),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Range,
                args: vec![IrExpr {
                    id: ExprId(41),
                    kind: IrExprKind::IntLit(5),
                }],
            },
        };
        let lowered = lowering.lower_expr(expr);
        match lowered.kind {
            IrExprKind::Range { start, end } => {
                assert!(matches!(start.kind, IrExprKind::IntLit(0)));
                assert!(matches!(end.kind, IrExprKind::IntLit(5)));
            }
            _ => panic!("Expected Range"),
        }
    }

    #[test]
    fn test_lower_range_two_args() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 600);
        let expr = IrExpr {
            id: ExprId(50),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Range,
                args: vec![
                    IrExpr {
                        id: ExprId(51),
                        kind: IrExprKind::IntLit(2),
                    },
                    IrExpr {
                        id: ExprId(52),
                        kind: IrExprKind::IntLit(4),
                    },
                ],
            },
        };
        let lowered = lowering.lower_expr(expr);
        match lowered.kind {
            IrExprKind::Range { start, end } => {
                assert!(matches!(start.kind, IrExprKind::IntLit(2)));
                assert!(matches!(end.kind, IrExprKind::IntLit(4)));
            }
            _ => panic!("Expected Range"),
        }
    }

    #[test]
    fn test_lower_range_three_args_keeps_builtin() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 700);
        let expr = IrExpr {
            id: ExprId(60),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Range,
                args: vec![
                    IrExpr {
                        id: ExprId(61),
                        kind: IrExprKind::IntLit(1),
                    },
                    IrExpr {
                        id: ExprId(62),
                        kind: IrExprKind::IntLit(5),
                    },
                    IrExpr {
                        id: ExprId(63),
                        kind: IrExprKind::IntLit(2),
                    },
                ],
            },
        };
        let lowered = lowering.lower_expr(expr);
        assert!(matches!(
            lowered.kind,
            IrExprKind::BuiltinCall {
                id: BuiltinId::Range,
                ..
            }
        ));
    }

    #[test]
    fn test_lower_any_empty_returns_false() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 800);
        let expr = IrExpr {
            id: ExprId(70),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Any,
                args: vec![],
            },
        };
        let lowered = lowering.lower_expr(expr);
        assert!(matches!(lowered.kind, IrExprKind::BoolLit(false)));
    }

    #[test]
    fn test_lower_all_empty_returns_true() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 900);
        let expr = IrExpr {
            id: ExprId(80),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::All,
                args: vec![],
            },
        };
        let lowered = lowering.lower_expr(expr);
        assert!(matches!(lowered.kind, IrExprKind::BoolLit(true)));
    }

    #[test]
    fn test_lower_sum_with_start_adds_binop() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 1000);
        let expr = IrExpr {
            id: ExprId(90),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Sum,
                args: vec![
                    IrExpr {
                        id: ExprId(91),
                        kind: IrExprKind::Var("xs".to_string()),
                    },
                    IrExpr {
                        id: ExprId(92),
                        kind: IrExprKind::IntLit(3),
                    },
                ],
            },
        };
        let lowered = lowering.lower_expr(expr);
        assert!(matches!(
            lowered.kind,
            IrExprKind::BinOp {
                op: IrBinOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn test_lower_int_any_uses_json_conversion() {
        let mut type_table = HashMap::new();
        type_table.insert(ExprId(101), Type::Any);
        let lowering = LoweringPass::new(HashMap::new(), type_table, 1100);
        let expr = IrExpr {
            id: ExprId(100),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Int,
                args: vec![IrExpr {
                    id: ExprId(101),
                    kind: IrExprKind::Var("v".to_string()),
                }],
            },
        };
        let lowered = lowering.lower_expr(expr);
        assert!(matches!(lowered.kind, IrExprKind::JsonConversion { .. }));
    }

    #[test]
    fn test_lower_float_non_any_uses_cast() {
        let mut type_table = HashMap::new();
        type_table.insert(ExprId(111), Type::Int);
        let lowering = LoweringPass::new(HashMap::new(), type_table, 1200);
        let expr = IrExpr {
            id: ExprId(110),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Float,
                args: vec![IrExpr {
                    id: ExprId(111),
                    kind: IrExprKind::Var("v".to_string()),
                }],
            },
        };
        let lowered = lowering.lower_expr(expr);
        assert!(matches!(lowered.kind, IrExprKind::Cast { .. }));
    }

    #[test]
    fn test_lower_list_empty_returns_list() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 1300);
        let expr = IrExpr {
            id: ExprId(120),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::List,
                args: vec![],
            },
        };
        let lowered = lowering.lower_expr(expr);
        assert!(matches!(lowered.kind, IrExprKind::List { .. }));
    }

    #[test]
    fn test_lower_bin_hex_oct_format() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 1500);
        let bin_expr = IrExpr {
            id: ExprId(140),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Bin,
                args: vec![IrExpr {
                    id: ExprId(141),
                    kind: IrExprKind::IntLit(2),
                }],
            },
        };
        let hex_expr = IrExpr {
            id: ExprId(142),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Hex,
                args: vec![IrExpr {
                    id: ExprId(143),
                    kind: IrExprKind::IntLit(2),
                }],
            },
        };
        let oct_expr = IrExpr {
            id: ExprId(144),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Oct,
                args: vec![IrExpr {
                    id: ExprId(145),
                    kind: IrExprKind::IntLit(2),
                }],
            },
        };
        let bin_lowered = lowering.lower_expr(bin_expr);
        let hex_lowered = lowering.lower_expr(hex_expr);
        let oct_lowered = lowering.lower_expr(oct_expr);
        assert!(matches!(bin_lowered.kind, IrExprKind::Call { .. }));
        assert!(matches!(hex_lowered.kind, IrExprKind::Call { .. }));
        assert!(matches!(oct_lowered.kind, IrExprKind::Call { .. }));
    }

    #[test]
    fn test_lower_chr_and_ord() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 1600);
        let chr_expr = IrExpr {
            id: ExprId(150),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Chr,
                args: vec![IrExpr {
                    id: ExprId(151),
                    kind: IrExprKind::IntLit(65),
                }],
            },
        };
        let ord_expr = IrExpr {
            id: ExprId(152),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Ord,
                args: vec![IrExpr {
                    id: ExprId(153),
                    kind: IrExprKind::StringLit("A".to_string()),
                }],
            },
        };
        let chr_lowered = lowering.lower_expr(chr_expr);
        let ord_lowered = lowering.lower_expr(ord_expr);
        assert!(matches!(
            chr_lowered.kind,
            IrExprKind::MethodCall { .. } | IrExprKind::Call { .. }
        ));
        assert!(matches!(ord_lowered.kind, IrExprKind::Cast { .. }));
    }

    #[test]
    fn test_lower_abs_min_max_round() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 1700);
        let abs_expr = IrExpr {
            id: ExprId(160),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Abs,
                args: vec![IrExpr {
                    id: ExprId(161),
                    kind: IrExprKind::IntLit(-1),
                }],
            },
        };
        let min_expr = IrExpr {
            id: ExprId(162),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Min,
                args: vec![
                    IrExpr {
                        id: ExprId(163),
                        kind: IrExprKind::IntLit(1),
                    },
                    IrExpr {
                        id: ExprId(164),
                        kind: IrExprKind::IntLit(2),
                    },
                ],
            },
        };
        let max_expr = IrExpr {
            id: ExprId(165),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Max,
                args: vec![
                    IrExpr {
                        id: ExprId(166),
                        kind: IrExprKind::IntLit(1),
                    },
                    IrExpr {
                        id: ExprId(167),
                        kind: IrExprKind::IntLit(2),
                    },
                ],
            },
        };
        let round_expr = IrExpr {
            id: ExprId(168),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Round,
                args: vec![IrExpr {
                    id: ExprId(169),
                    kind: IrExprKind::FloatLit(1.2),
                }],
            },
        };
        assert!(matches!(
            lowering.lower_expr(abs_expr).kind,
            IrExprKind::MethodCall { .. }
                | IrExprKind::Call { .. }
                | IrExprKind::BuiltinCall { .. }
        ));
        assert!(matches!(
            lowering.lower_expr(min_expr).kind,
            IrExprKind::MethodCall { .. }
                | IrExprKind::Call { .. }
                | IrExprKind::BuiltinCall { .. }
        ));
        assert!(matches!(
            lowering.lower_expr(max_expr).kind,
            IrExprKind::MethodCall { .. }
                | IrExprKind::Call { .. }
                | IrExprKind::BuiltinCall { .. }
        ));
        assert!(matches!(
            lowering.lower_expr(round_expr).kind,
            IrExprKind::MethodCall { .. }
                | IrExprKind::Call { .. }
                | IrExprKind::BuiltinCall { .. }
        ));
    }

    #[test]
    fn test_lower_tuple_from_listcomp_passthrough() {
        let lowering = LoweringPass::new(HashMap::new(), HashMap::new(), 1400);
        let expr = IrExpr {
            id: ExprId(130),
            kind: IrExprKind::BuiltinCall {
                id: BuiltinId::Tuple,
                args: vec![IrExpr {
                    id: ExprId(131),
                    kind: IrExprKind::ListComp {
                        elt: Box::new(IrExpr {
                            id: ExprId(132),
                            kind: IrExprKind::Var("x".to_string()),
                        }),
                        target: "x".to_string(),
                        iter: Box::new(IrExpr {
                            id: ExprId(133),
                            kind: IrExprKind::Var("xs".to_string()),
                        }),
                        condition: None,
                    },
                }],
            },
        };
        let lowered = lowering.lower_expr(expr);
        assert!(matches!(lowered.kind, IrExprKind::ListComp { .. }));
    }
}
