//! Expression analysis for SemanticAnalyzer
//!
//! Extracted from mod.rs for maintainability

use super::operators::convert_binop;
use crate::ir::BuiltinId;
// use super::type_infer::TypeInference;
use super::*;

impl SemanticAnalyzer {
    /// 名前解決された関数名を取得 (e.g., "len", "pd.read_csv")
    fn get_call_name(&self, func: &Expr) -> Option<String> {
        match func {
            Expr::Ident(name) => Some(name.clone()),
            Expr::Attribute { value, attr } => {
                if let Some(mut base) = self.get_call_name(value) {
                    base.push('.');
                    base.push_str(attr);
                    Some(base)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn analyze_expr(&mut self, expr: &Expr) -> Result<IrExpr, TsuchinokoError> {
        match expr {
            Expr::IntLiteral(n) => Ok(self.create_expr(IrExprKind::IntLit(*n), Type::Int)),
            Expr::FloatLiteral(f) => Ok(self.create_expr(IrExprKind::FloatLit(*f), Type::Float)),
            Expr::StringLiteral(s) => Ok(self.create_expr(IrExprKind::StringLit(s.clone()), Type::String)),
            Expr::BoolLiteral(b) => Ok(self.create_expr(IrExprKind::BoolLit(*b), Type::Bool)),
            Expr::NoneLiteral => Ok(self.create_expr(IrExprKind::NoneLit, Type::Unknown)),
            Expr::Ident(name) => {
                // Check for scope-crossing variable usage (for hoisting detection)
                let current_depth = self.scope.depth();
                if let Some(var_info) = self.scope.lookup(name) {
                    let defined_depth = var_info.defined_at_depth;
                    let var_ty = var_info.ty.clone();

                    // If variable is used at a shallower depth than where it was defined,
                    // it needs to be hoisted (defined in inner block, used in outer scope)
                    if current_depth < defined_depth {
                        self.hoisted_var_candidates
                            .entry(name.clone())
                            .and_modify(|(_, _, used)| {
                                // Keep track of the shallowest usage depth
                                if current_depth < *used {
                                    *used = current_depth;
                                }
                            })
                            .or_insert((var_ty.clone(), defined_depth, current_depth));
                    }
                }

                // Check if this variable has a narrowed type
                // If the original type is Optional<T> but it's narrowed to T, we need to unwrap
                if let Some(original_info) = self.scope.lookup(name) {
                    let original_ty = original_info.ty.clone();
                    if let Some(narrowed_ty) = self.scope.get_effective_type(name) {
                        // If original is Optional<T> and narrowed is T, emit Unwrap
                        if let Type::Optional(inner) = &original_ty {
                            if *inner.as_ref() == narrowed_ty {
                                let var_expr = self.create_expr(IrExprKind::Var(name.clone()), original_ty);
                                return Ok(self.create_expr(IrExprKind::Unwrap(Box::new(var_expr)), narrowed_ty));
                            }
                        }
                    }
                }
                // V1.7.0: Refactor - Check if this is a module alias (Triple Hybrid)
                if let Some(real_target) = self.module_global_aliases.get(name) {
                    match crate::bridge::module_table::get_import_mode(real_target) {
                        crate::bridge::module_table::ImportMode::Native => {
                            // Rust native - check if it's a constant or the module itself
                            if let Some(code) =
                                crate::bridge::module_table::generate_native_code(real_target, &[])
                            {
                                return Ok(self.create_expr(IrExprKind::RawCode(code), Type::Any));
                            }
                            // Module identifier itself - keep as Var for attribute access
                            return Ok(self.create_expr(IrExprKind::Var(name.clone()), Type::Any));
                        }
                        _ => {
                            // Non-native (PyO3 or Resident) - requires BridgeGet
                            self.current_func_needs_bridge = true;
                            return Ok(self.create_expr(IrExprKind::BridgeGet {
                                alias: name.clone(),
                            }, Type::Any));
                        }
                    }
                }

                let var_ty = self.infer_type(expr);
                Ok(self.create_expr(IrExprKind::Var(name.clone()), var_ty))
            }
            Expr::BinOp { left, op, right } => {
                // Handle 'in' operator: x in y -> y.contains(&x) or y.contains_key(&x)
                if let AstBinOp::In = op {
                    let mut right_ty = self.infer_type(right);
                    // Unwrap Ref to get inner type for dict vs list detection
                    while let Type::Ref(inner) = right_ty {
                        right_ty = *inner;
                    }
                    let ir_left = self.analyze_expr(left)?;
                    let ir_right = self.analyze_expr(right)?;

                    let method = match right_ty {
                        Type::List(_) | Type::Tuple(_) | Type::Unknown => "contains", // Default to contains for unknown (Vec assumed)
                        Type::Dict(_, _) => "contains_key",
                        _ => "contains",
                    };

                    let left_ty = self.infer_type(left);
                    let arg = if matches!(left_ty, Type::Ref(_) | Type::String) {
                        // Already a reference type, don't add another &
                        ir_left
                    } else {
                        self.create_expr(IrExprKind::Reference {
                            target: Box::new(ir_left),
                        }, Type::Ref(Box::new(left_ty)))
                    };

                    return Ok(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_right),
                        method: method.to_string(),
                        args: vec![arg],
                        callee_needs_bridge: false,
                    }, Type::Bool));
                }

                // Handle 'not in' operator: x not in y -> !y.contains(&x) or !y.contains_key(&x) (V1.3.0)
                if let AstBinOp::NotIn = op {
                    let mut right_ty = self.infer_type(right);
                    // Unwrap Ref to get inner type for dict vs list detection
                    while let Type::Ref(inner) = right_ty {
                        right_ty = *inner;
                    }
                    let ir_left = self.analyze_expr(left)?;
                    let ir_right = self.analyze_expr(right)?;

                    let method = match right_ty {
                        Type::List(_) | Type::Tuple(_) | Type::Unknown => "contains",
                        Type::Dict(_, _) => "contains_key",
                        Type::String => "contains",
                        _ => "contains",
                    };

                    let left_ty = self.infer_type(left);
                    let arg = if matches!(left_ty, Type::Ref(_) | Type::String) {
                        // Already a reference type, don't add another &
                        ir_left
                    } else {
                        self.create_expr(IrExprKind::Reference {
                            target: Box::new(ir_left),
                        }, Type::Ref(Box::new(left_ty)))
                    };
                    let contains_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_right),
                        method: method.to_string(),
                        args: vec![arg],
                        callee_needs_bridge: false,
                    }, Type::Bool);

                    return Ok(self.create_expr(IrExprKind::UnaryOp {
                        op: IrUnaryOp::Not,
                        operand: Box::new(contains_call),
                    }, Type::Bool));
                }

                // Handle 'is' and 'is not' operators with None
                if let AstBinOp::Is | AstBinOp::IsNot = op {
                    // Check if right side is None
                    if let Expr::NoneLiteral = right.as_ref() {
                        let left_ty = self.infer_type(left);
                        let ir_left = self.analyze_expr(left)?;

                        match left_ty {
                            Type::Optional(_) => {
                                // Optional type: use is_some()/is_none()
                                let method = if matches!(op, AstBinOp::Is) {
                                    "is_none"
                                } else {
                                    "is_some"
                                };
                                return Ok(self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(ir_left),
                                    method: method.to_string(),
                                    args: vec![],
                                    callee_needs_bridge: false,
                                }, Type::Bool));
                            }
                            _ => {
                                // Non-Optional type: always true/false
                                // Use RawCode to include warning comment
                                let (value, warning) = if matches!(op, AstBinOp::Is) {
                                    ("false", "/* Warning: 'is None' on non-Optional type */")
                                } else {
                                    ("true", "/* Warning: 'is not None' on non-Optional type */")
                                };
                                let code = format!("{} {}", value, warning);
                                return Ok(self.create_expr(IrExprKind::RawCode(code), Type::Bool));
                            }
                        }
                    }
                }

                // V1.5.0: Set operations - |, &, - on set types
                // a | b -> a.union(&b).cloned().collect()
                // a & b -> a.intersection(&b).cloned().collect()
                // a - b -> a.difference(&b).cloned().collect()
                if matches!(op, AstBinOp::BitOr | AstBinOp::BitAnd | AstBinOp::Sub) {
                    let left_ty = self.infer_type(left);
                    let right_ty = self.infer_type(right);

                    // Check if both operands are set types
                    if matches!(left_ty, Type::Set(_)) && matches!(right_ty, Type::Set(_)) {
                        let ir_left = self.analyze_expr(left)?;
                        let ir_right = self.analyze_expr(right)?;

                        let method = match op {
                            AstBinOp::BitOr => "union",
                            AstBinOp::BitAnd => "intersection",
                            AstBinOp::Sub => "difference",
                            _ => unreachable!(),
                        };

                        // Generate: left.method(&right).cloned().collect()
                        let ref_right = self.create_expr(IrExprKind::Reference {
                            target: Box::new(ir_right),
                        }, Type::Ref(Box::new(right_ty.clone())));

                        let method_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(ir_left),
                            method: method.to_string(),
                            args: vec![ref_right],
                            callee_needs_bridge: false,
                        }, Type::Unknown); // Intermediate iterator type

                        let cloned_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(method_call),
                            method: "cloned".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);

                        // Use collect_hashset marker for type inference
                        return Ok(self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(cloned_call),
                            method: "collect_hashset".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, left_ty)); // Result is same set type as left
                    }
                }

                // V1.5.0: 'or' with Optional type -> unwrap_or
                // x or default -> x.unwrap_or(default)
                if matches!(op, AstBinOp::Or) {
                    let left_ty = self.infer_type(left);
                    if matches!(left_ty, Type::Optional(_)) {
                        let ir_left = self.analyze_expr(left)?;
                        let ir_right = self.analyze_expr(right)?;
                        // If right is StringLit, add .to_string()
                        let ir_right = if matches!(ir_right.kind, IrExprKind::StringLit(_)) {
                            self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_right),
                                method: "to_string".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::String)
                        } else {
                            ir_right
                        };

                        return Ok(self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(ir_left),
                            method: "unwrap_or".to_string(),
                            args: vec![ir_right],
                            callee_needs_bridge: false,
                        }, self.infer_type(expr)));
                    }

                    // V1.5.0: 'or' with empty String falsy behavior
                    // x or default -> if x.is_empty() { default } else { x.clone() }
                    if matches!(left_ty, Type::String) {
                        let ir_left = self.analyze_expr(left)?;
                        let ir_right = self.analyze_expr(right)?;
                        let ir_right = if matches!(ir_right.kind, IrExprKind::StringLit(_)) {
                            self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_right),
                                method: "to_string".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::String)
                        } else {
                            ir_right
                        };

                        let is_empty_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::String,
                            target: Box::new(ir_left.clone()),
                            method: "is_empty".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Bool);

                        let left_clone = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::String,
                            target: Box::new(ir_left),
                            method: "clone".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::String);

                        return Ok(self.create_expr(IrExprKind::IfExp {
                            test: Box::new(is_empty_call),
                            body: Box::new(ir_right),
                            orelse: Box::new(left_clone),
                        }, Type::String));
                    }
                }

                let ir_left = self.analyze_expr(left)?;
                let ir_right = self.analyze_expr(right)?;
                let ir_op = convert_binop(op);

                // V1.6.0: 比較演算で左辺がfloat、右辺がintリテラルの場合に右辺をfloatに変換
                // Python: `value < 0` (value: float) → Rust: `value < 0.0f64`
                let ir_right = if matches!(
                    op,
                    AstBinOp::Lt
                        | AstBinOp::Gt
                        | AstBinOp::LtEq
                        | AstBinOp::GtEq
                        | AstBinOp::Eq
                        | AstBinOp::NotEq
                ) {
                    let left_ty = self.infer_type(left);
                    if matches!(left_ty, Type::Float) {
                        if let IrExprKind::IntLit(n) = ir_right.kind {
                            self.create_expr(IrExprKind::FloatLit(n as f64), Type::Float)
                        } else {
                            ir_right
                        }
                    } else {
                        ir_right
                    }
                } else {
                    ir_right
                };

                let res_ty = self.infer_type(expr);
                Ok(self.create_expr(IrExprKind::BinOp {
                    left: Box::new(ir_left),
                    op: ir_op,
                    right: Box::new(ir_right),
                }, res_ty))
            }
            Expr::Call { func, args, kwargs } => {
                // 1. Builtin Table Lookup
                if let Some(fullname) = self.get_call_name(func) {
                    // sorted や print は一旦除外（特殊処理が多いため）
                    if fullname != "print" && fullname != "sorted" {
                        if let Some(spec) = crate::bridge::builtin_table::get_builtin_spec(&fullname) {
                            let ir_args = args.iter().map(|a| self.analyze_expr(a)).collect::<Result<Vec<_>, _>>()?;
                            let arg_types: Vec<Type> = args.iter().map(|a| self.infer_type(a)).collect();
                            
                            let ret_ty = (spec.ret_ty_resolver)(&arg_types);
                            
                            return Ok(self.create_expr(IrExprKind::BuiltinCall {
                                id: spec.id,
                                args: ir_args,
                            }, ret_ty));
                        }
                    }
                }
                // Handle print() calls with type information for proper formatting
                if let Expr::Ident(name) = func.as_ref() {
                    if name == "print" {
                        let typed_args: Result<Vec<(IrExpr, Type)>, TsuchinokoError> = args
                            .iter()
                            .map(|a| {
                                let ir_arg = self.analyze_expr(a)?;
                                let raw_ty = self.infer_type(a);
                                Ok((ir_arg, self.resolve_type(&raw_ty)))
                            })
                            .collect();
                        return Ok(self.create_expr(IrExprKind::Print { args: typed_args? }, Type::Unit));
                    }

                    // V1.3.0: sorted(iterable) or sorted(iterable, reverse=True) or sorted(iterable, key=lambda)
                    if name == "sorted" && !args.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;

                        // Check for key argument
                        let key_lambda = kwargs.iter().find(|(k, _)| k == "key").map(|(_, v)| v);

                        // Check for reverse argument
                        let reverse = kwargs
                            .iter()
                            .find(|(k, _)| k == "reverse")
                            .map(|(_, v)| matches!(v, Expr::BoolLiteral(true)))
                            .unwrap_or(false);

                        if let Some(Expr::Lambda { params, body }) = key_lambda {
                            // sorted(iterable, key=lambda x: expr) -> sort_by_key
                            let param = params.first().cloned().unwrap_or_else(|| "x".to_string());
                            // Phase E: Register lambda param in scope before analyzing body
                            self.scope.push();
                            self.scope.define(&param, Type::Unknown, false);
                            let ir_body = self.analyze_expr(body)?;
                            self.scope.pop();
                            let body_str = self.emit_simple_ir_expr(&ir_body);

                            let res_ty = self.infer_type(expr);
                            if reverse {
                                return Ok(self.create_expr(IrExprKind::RawCode(format!(
                                    "{{ let mut v = {}.to_vec(); v.sort_by(|a, b| {{ let {} = b; {} }}.cmp(&{{ let {} = a; {} }})); v }}",
                                    self.emit_simple_ir_expr(&ir_arg),
                                    param, body_str, param, body_str
                                )), res_ty));
                            } else {
                                return Ok(self.create_expr(IrExprKind::RawCode(format!(
                                    "{{ let mut v = {}.to_vec(); v.sort_by_key(|{}| {}); v }}",
                                    self.emit_simple_ir_expr(&ir_arg),
                                    param,
                                    body_str
                                )), res_ty));
                            }
                        } else if reverse {
                            return Ok(self.create_expr(IrExprKind::RawCode(format!(
                                "{{ let mut v = {}.to_vec(); v.sort_by(|a, b| b.cmp(a)); v }}",
                                self.emit_simple_ir_expr(&ir_arg)
                            )), self.infer_type(expr)));
                        } else {
                            return Ok(self.create_expr(IrExprKind::RawCode(format!(
                                "{{ let mut v = {}.to_vec(); v.sort(); v }}",
                                self.emit_simple_ir_expr(&ir_arg)
                            )), self.infer_type(expr)));
                        }
                    }


                    // V1.3.0: list(something) - if something is map/filter, we need special handling
                    if name == "list" && args.len() == 1 && kwargs.is_empty() {
                        // Check if inner is map() or filter()
                        if let Expr::Call {
                            func: inner_func,
                            args: inner_args,
                            kwargs: inner_kwargs,
                        } = &args[0]
                        {
                            if let Expr::Ident(inner_name) = inner_func.as_ref() {
                                // list(map(f, iter)) -> iter.iter().map(f).cloned().collect()
                                if inner_name == "map"
                                    && inner_args.len() == 2
                                    && inner_kwargs.is_empty()
                                {
                                    let lambda = &inner_args[0];
                                    let iterable = &inner_args[1];
                                    let ir_iter = self.analyze_expr(iterable)?;
                                    let ir_lambda = self.analyze_expr(lambda)?;
                                    let ir_lambda = if let IrExprKind::BoxNew(inner) = ir_lambda.kind {
                                        *inner
                                    } else {
                                        ir_lambda
                                    };

                                    let iter_call = self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(ir_iter),
                                        method: "iter".to_string(),
                                        args: vec![],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown);
                                    let map_call = self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(iter_call),
                                        method: "map".to_string(),
                                        args: vec![ir_lambda],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown);
                                    return Ok(self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(map_call),
                                        method: "collect".to_string(),
                                        args: vec![],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown));
                                }

                                // list(filter(f, iter)) -> iter.iter().cloned().filter(|x| cond).collect()
                                if inner_name == "filter"
                                    && inner_args.len() == 2
                                    && inner_kwargs.is_empty()
                                {
                                    let lambda = &inner_args[0];
                                    let iterable = &inner_args[1];
                                    let ir_iter = self.analyze_expr(iterable)?;

                                    // For filter, |&x| pattern dereferences the reference
                                    let filter_closure =
                                        if let Expr::Lambda { params, body } = lambda {
                                            if params.len() == 1 {
                                                let param = &params[0];
                                                // Phase E: Register lambda param in scope before analyzing body
                                                self.scope.push();
                                                self.scope.define(param, Type::Unknown, false);
                                                let body_ir = self.analyze_expr(body)?;
                                                self.scope.pop();
                                                self.create_expr(IrExprKind::RawCode(format!(
                                                    "|&{}| {}",
                                                    param,
                                                    self.emit_simple_ir_expr(&body_ir)
                                                )), Type::Unknown)
                                            } else {
                                                self.analyze_expr(lambda)?
                                            }
                                        } else {
                                            let ir_lambda = self.analyze_expr(lambda)?;
                                            if let IrExprKind::BoxNew(inner) = ir_lambda.kind {
                                                *inner
                                            } else {
                                                ir_lambda
                                            }
                                        };

                                    let iter_call = self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(ir_iter),
                                        method: "iter".to_string(),
                                        args: vec![],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown);
                                    // cloned() before filter to get owned values
                                    let cloned_call = self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(iter_call),
                                        method: "cloned".to_string(),
                                        args: vec![],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown);
                                    let filter_call = self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(cloned_call),
                                        method: "filter".to_string(),
                                        args: vec![filter_closure],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown);
                                    return Ok(self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(filter_call),
                                        method: "collect".to_string(),
                                        args: vec![],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown));
                                }
                            }
                        }
                    }

                    // V1.5.0: set(iterable) -> iterable.iter().cloned().collect::<HashSet<_>>()
                    if name == "set" && args.len() == 1 && kwargs.is_empty() {
                        let arg = &args[0];
                        let ir_arg = self.analyze_expr(arg)?;

                        // Build: arg.iter().cloned().collect()
                        // Use MethodCall chain, emitter will handle turbofish
                        let iter_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(ir_arg),
                            method: "iter".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        let cloned_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(iter_call),
                            method: "cloned".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        // Special marker for set collect - emitter will add turbofish
                        return Ok(self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(cloned_call),
                            method: "collect_hashset".to_string(), // Special marker
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown));
                    }
                }

                // V1.6.0 FT-002: Handle super().method() -> self.base.method()
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    // Check if value is super() call
                    if let Expr::Call {
                        func: super_func,
                        args: super_args,
                        ..
                    } = value.as_ref()
                    {
                        if let Expr::Ident(super_name) = super_func.as_ref() {
                            if super_name == "super" && super_args.is_empty() {
                                // This is super().method(...) pattern
                                // Transform to self.base.method(...)
                                let ir_args: Vec<IrExpr> = args
                                    .iter()
                                    .map(|a| self.analyze_expr(a))
                                    .collect::<Result<Vec<_>, _>>()?;

                                // self.base
                                let self_var = self.create_expr(IrExprKind::Var("self".to_string()), Type::Unknown);
                                let base_access = self.create_expr(IrExprKind::FieldAccess {
                                    target: Box::new(self_var),
                                    field: "base".to_string(),
                                }, Type::Unknown);

                                // self.base.method(args)
                                return Ok(self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(base_access),
                                    method: attr.clone(),
                                    args: ir_args,
                                    callee_needs_bridge: false,
                                }, Type::Unknown));
                            }
                        }
                    }
                }

                // V1.7.0: Handle method calls on Any types (BridgeMethodCall)
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    let value_ty = self.infer_type(value);
                    if matches!(value_ty, Type::Any) {
                        let ir_target = self.analyze_expr(value)?;
                        let ir_args: Vec<IrExpr> = args
                            .iter()
                            .map(|a| {
                                let arg_ty = self.infer_type(a);
                                let ir_arg = self.analyze_expr(a)?;
                                if matches!(arg_ty, Type::Any) {
                                    // Zero-Copy: Pass reference to existing TnkValue
                                    Ok::<IrExpr, crate::error::TsuchinokoError>(self.create_expr(IrExprKind::Ref(Box::new(ir_arg)), Type::Any))
                                } else {
                                    // Conversion: Create refined TnkValue then pass reference
                                    let tnk_val = self.create_expr(IrExprKind::TnkValueFrom(Box::new(ir_arg)), Type::Any);
                                    Ok::<IrExpr, crate::error::TsuchinokoError>(self.create_expr(IrExprKind::Ref(Box::new(tnk_val)), Type::Any))
                                }
                            })
                            .collect::<Result<Vec<IrExpr>, crate::error::TsuchinokoError>>()?;

                        // Analyze keyword args
                        let mut ir_kwargs = Vec::new();
                        for (k, v) in kwargs {
                            let ir_v = self.analyze_expr(v)?;
                            let arg_ty = self.infer_type(v);
                            let wrapped_ir = if matches!(arg_ty, Type::Any) {
                                self.create_expr(IrExprKind::Ref(Box::new(ir_v)), Type::Any)
                            } else {
                                let tnk_val = self.create_expr(IrExprKind::TnkValueFrom(Box::new(ir_v)), Type::Any);
                                self.create_expr(IrExprKind::Ref(Box::new(tnk_val)), Type::Any)
                            };
                            ir_kwargs.push((k.clone(), wrapped_ir));
                        }
 
                        self.current_func_may_raise = true;
 
                        return Ok(self.create_expr(IrExprKind::BridgeMethodCall {
                            target: Box::new(ir_target),
                            method: attr.clone(),
                            args: ir_args,
                            keywords: ir_kwargs,
                        }, Type::Any));
                    }
                }

                // Handle PyO3 module calls: np.array(...) -> np.call_method1("array", (...))?
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Expr::Ident(module_alias) = value.as_ref() {
                        // V1.7.0: Triple Hybrid System (Native, PyO3, Bridge)
                        // PyO3 Table: Whitelist of modules supported via PyO3 (Direct FFI)
                        // Currently empty as per V1.7.0 requirements.
                        const PYO3_SUPPORTED_MODULES: &[&str] = &[]; // e.g. &["numpy", "pandas"] if supported

                        // Check if this is a PyO3 import alias
                        let is_pyo3_module = self
                            .external_imports
                            .iter()
                            .any(|(real_name, alias)| {
                                alias == module_alias && PYO3_SUPPORTED_MODULES.contains(&real_name.as_str())
                            });

                        if is_pyo3_module {
                            // Convert to PyO3 call
                            let ir_args: Vec<IrExpr> = args
                                .iter()
                                .map(|a| self.analyze_expr(a))
                                .collect::<Result<Vec<_>, _>>()?;

                            // V1.5.2: PyO3 calls can fail, mark current function as may_raise
                            self.current_func_may_raise = true;

                            // Return structured PyO3 call
                            return Ok(self.create_expr(IrExprKind::PyO3Call {
                                module: module_alias.clone(),
                                method: attr.clone(),
                                args: ir_args,
                            }, Type::Any));
                        }
                    }
                }

                // Handle static method calls: ClassName.method() -> ClassName::method()
                // Handle static method calls: ClassName.method() -> ClassName::method()
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Expr::Ident(class_name) = value.as_ref() {
                        // Check if this is a known struct (class)
                        if self.struct_field_types.contains_key(class_name)
                            || class_name
                                .chars()
                                .next()
                                .map(|c| c.is_uppercase())
                                .unwrap_or(false)
                        {
                            // Static method call: ClassName.method() -> ClassName::method()
                            let ir_args: Vec<IrExpr> = args
                                .iter()
                                .map(|a| self.analyze_expr(a))
                                .collect::<Result<Vec<_>, _>>()?;

                            // Generate raw code for static call with ::
                            let args_str = if ir_args.is_empty() {
                                String::new()
                            } else {
                                // We'll handle args in the IrExpr::Call
                                let raw_code = self.create_expr(IrExprKind::RawCode(format!(
                                    "{class_name}::{attr}"
                                )), Type::Unknown);
                                return Ok(self.create_expr(IrExprKind::Call {
                                    func: Box::new(raw_code),
                                    args: ir_args,
                                    callee_may_raise: false,
                                    callee_needs_bridge: false,
                                }, Type::Unknown));
                            };
                            return Ok(self.create_expr(IrExprKind::RawCode(format!(
                                "{class_name}::{attr}({args_str})"
                            )), Type::Unknown));
                        }
                    }
                }

                // V1.4.0: Handle native module functions/constants (Triple Hybrid)
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Expr::Ident(module_name) = value.as_ref() {
                        // Resolve module alias (e.g., m -> math)
                        let real_module = self
                            .module_global_aliases
                            .get(module_name)
                            .map(|s| s.as_str())
                            .unwrap_or(module_name);

                        let full_target = format!("{real_module}.{attr}");
                        let mut ir_args = Vec::new();
                        for a in args {
                            ir_args.push(self.analyze_expr(a)?);
                        }

                        if let Some(binding) =
                            crate::bridge::module_table::get_native_binding(&full_target)
                        {
                            match binding {
                                crate::bridge::module_table::NativeBinding::Method(rust_method) => {
                                    if !ir_args.is_empty() && kwargs.is_empty() {
                                        return Ok(self.create_expr(IrExprKind::MethodCall {
                                            target_type: Type::Unknown,
                                            target: Box::new(ir_args[0].clone()),
                                            method: rust_method.to_string(),
                                            args: ir_args[1..].to_vec(),
                                            callee_needs_bridge: false,
                                        }, Type::Unknown));
                                    }
                                }
                                crate::bridge::module_table::NativeBinding::Constant(code) => {
                                    if ir_args.is_empty() && kwargs.is_empty() {
                                        return Ok(self.create_expr(IrExprKind::RawCode(code.to_string()), Type::Unknown));
                                    }
                                }
                            }
                        }
                    }
                }

                if let Expr::Attribute { value: _, attr } = func.as_ref() {
                    if attr == "items" && args.is_empty() && kwargs.is_empty() {
                        // Convert .items() to .iter() for HashMap
                        // Unwrap matches structure of Expr::Attribute.
                        if let Expr::Attribute { value, .. } = *func.clone() {
                            let ir_target = self.analyze_expr(&value)?;
                            return Ok(self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_target),
                                method: "iter".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown));
                        }
                    }
                }

                // Build ordered argument list using func_param_info
                // 1. Start with positional args
                // 2. Match kwargs by parameter name
                // 3. Fill missing args with default values
                // 4. Handle variadic parameters (*args)
                let resolved_args: Vec<Expr> = match func.as_ref() {
                    Expr::Ident(name) => {
                        if let Some(param_info) = self.func_param_info.get(name) {
                            // Check if the last (or any) parameter is variadic
                            let variadic_idx = param_info
                                .iter()
                                .position(|(_, _, _, is_variadic)| *is_variadic);

                            if let Some(var_idx) = variadic_idx {
                                // Handle variadic function call
                                let non_variadic_count = var_idx;
                                let mut result: Vec<Expr> = Vec::new();

                                // Fill non-variadic positional args
                                for (i, arg) in args.iter().enumerate() {
                                    if i < non_variadic_count {
                                        result.push(arg.clone());
                                    }
                                }

                                // Fill defaults for missing non-variadic args
                                for i in result.len()..non_variadic_count {
                                    if let Some((_, _, Some(default_expr), _)) = param_info.get(i) {
                                        result.push(default_expr.clone());
                                    }
                                }

                                // Collect remaining args for the variadic parameter
                                let variadic_args: Vec<Expr> =
                                    args.iter().skip(non_variadic_count).cloned().collect();

                                // Check if there's a single starred argument (e.g., *nums)
                                // In this case, pass it directly instead of wrapping in a list
                                if variadic_args.len() == 1 {
                                    if let Expr::Starred(inner) = &variadic_args[0] {
                                        // Starred expression - pass the inner expression directly
                                        result.push(*inner.clone());
                                    } else {
                                        // Single non-starred arg - wrap in list
                                        result.push(Expr::List(variadic_args));
                                    }
                                } else if variadic_args
                                    .iter()
                                    .any(|a| matches!(a, Expr::Starred(_)))
                                {
                                    // Mixed starred and non-starred - for now, just use the args
                                    // TODO: Handle more complex cases
                                    for arg in variadic_args {
                                        if let Expr::Starred(inner) = arg {
                                            result.push(*inner);
                                        } else {
                                            result.push(arg);
                                        }
                                    }
                                } else {
                                    // Create a List expression for the variadic args
                                    result.push(Expr::List(variadic_args));
                                }

                                result
                            } else {
                                // Non-variadic function - normal handling
                                let mut result: Vec<Option<Expr>> = vec![None; param_info.len()];

                                // Fill positional args
                                for (i, arg) in args.iter().enumerate() {
                                    if i < result.len() {
                                        result[i] = Some(arg.clone());
                                    }
                                }

                                // Fill kwargs by parameter name
                                for (kwarg_name, kwarg_value) in kwargs {
                                    if let Some(pos) = param_info
                                        .iter()
                                        .position(|(pname, _, _, _)| pname == kwarg_name)
                                    {
                                        result[pos] = Some(kwarg_value.clone());
                                    }
                                }

                                // Fill defaults for any remaining None values
                                for (i, slot) in result.iter_mut().enumerate() {
                                    if slot.is_none() {
                                        if let Some((_, _, Some(default_expr), _)) =
                                            param_info.get(i)
                                        {
                                            *slot = Some(default_expr.clone());
                                        }
                                    }
                                }

                                // Collect non-None values (skip trailing None if function allows)
                                result.into_iter().flatten().collect()
                            }
                        } else {
                            // No param info available - fall back to simple concatenation
                            let mut all: Vec<Expr> = args.clone();
                            for (_, value) in kwargs {
                                all.push(value.clone());
                            }
                            all
                        }
                    }
                    _ => {
                        // Non-ident function call - simple concatenation
                        let mut all: Vec<Expr> = args.clone();
                        for (_, value) in kwargs {
                            all.push(value.clone());
                        }
                        all
                    }
                };

                match func.as_ref() {
                    Expr::Ident(name) => {
                        // Try built-in function handler first
                        if let Some(ir_expr) = self.try_handle_builtin_call(name, &resolved_args)? {
                            return Ok(ir_expr);
                        }

                        // Check if this is a struct constructor call
                        if let Some(field_types) = self.struct_field_types.get(name).cloned() {
                            // V1.3.1: Generate IrExpr::StructConstruct instead of IrExpr::Call
                            // This moves the struct construction responsibility from emitter to semantic
                            let expected_types: Vec<Type> =
                                field_types.iter().map(|(_, ty)| ty.clone()).collect();
                            let field_names: Vec<String> =
                                field_types.iter().map(|(name, _)| name.clone()).collect();
                            let ir_args =
                                self.analyze_call_args(&resolved_args, &expected_types, name)?;

                            // Build field list with names and values
                            // V1.5.2: If no arguments provided but fields exist, use default values from __init__
                            let fields: Vec<(String, IrExpr)> = if ir_args.is_empty()
                                && !field_names.is_empty()
                            {
                                // Get default values from struct_field_defaults (populated from __init__)
                                let defaults = self
                                    .struct_field_defaults
                                    .get(name)
                                    .cloned()
                                    .unwrap_or_default();
                                let defaults_map: std::collections::HashMap<_, _> =
                                    defaults.into_iter().collect();

                                field_types
                                    .iter()
                                    .map(|(field_name, ty)| {
                                        // Use actual default value from __init__ if available
                                        let default_val =
                                            if let Some(ir) = defaults_map.get(field_name) {
                                                ir.clone()
                                            } else {
                                                // Fallback to type-based default (should rarely happen)
                                                match ty {
                                                    Type::Int => self.create_expr(IrExprKind::IntLit(0), Type::Int),
                                                    Type::Float => self.create_expr(IrExprKind::FloatLit(0.0), Type::Float),
                                                    Type::Bool => self.create_expr(IrExprKind::BoolLit(false), Type::Bool),
                                                    Type::String => {
                                                        let empty_str = self.create_expr(IrExprKind::StringLit(
                                                            String::new(),
                                                        ), Type::String);
                                                        self.create_expr(IrExprKind::MethodCall {
                                                            target_type: Type::Unknown,
                                                            target: Box::new(empty_str),
                                                            method: "to_string".to_string(),
                                                            args: vec![],
                                                            callee_needs_bridge: false,
                                                        }, Type::String)
                                                    }
                                                    _ => self.create_expr(IrExprKind::IntLit(0), Type::Int),
                                                }
                                            };
                                        (field_name.clone(), default_val)
                                    })
                                    .collect()
                            } else if let Some(parent_name) = self.struct_bases.get(name).cloned() {
                                // V1.6.0: If this struct has a base class, transform base field
                                // Dog("Rex", "Labrador") -> Dog { base: Animal { name: "Rex" }, breed: "Labrador" }
                                if let Some(parent_field_types) =
                                    self.struct_field_types.get(&parent_name).cloned()
                                {
                                    // Get parent field names (excluding base)
                                    let parent_fields: Vec<String> = parent_field_types
                                        .iter()
                                        .filter(|(n, _)| n != "base")
                                        .map(|(n, _)| n.clone())
                                        .collect();
                                    let parent_count = parent_fields.len();

                                    // Split args: first N go to parent, rest to child
                                    let (parent_args, child_args): (Vec<_>, Vec<_>) = ir_args
                                        .into_iter()
                                        .enumerate()
                                        .partition(|(i, _)| *i < parent_count);

                                    // Create parent struct
                                    let parent_struct = self.create_expr(IrExprKind::StructConstruct {
                                        name: parent_name.clone(),
                                        fields: parent_fields
                                            .into_iter()
                                            .zip(parent_args.into_iter().map(|(_, v)| v))
                                            .collect(),
                                    }, Type::Struct(parent_name.clone()));

                                    // Build child fields with base = parent struct
                                    let mut result_fields =
                                        vec![("base".to_string(), parent_struct)];

                                    // Child's own fields (excluding base)
                                    let child_field_names: Vec<String> = field_names
                                        .iter()
                                        .filter(|n| *n != "base")
                                        .cloned()
                                        .collect();
                                    result_fields.extend(
                                        child_field_names
                                            .into_iter()
                                            .zip(child_args.into_iter().map(|(_, v)| v)),
                                    );
                                    result_fields
                                } else {
                                    field_names.into_iter().zip(ir_args).collect()
                                }
                            } else {
                                field_names.into_iter().zip(ir_args).collect()
                            };

                            return Ok(self.create_expr(IrExprKind::StructConstruct {
                                name: name.clone(),
                                fields,
                            }, Type::Struct(name.clone())));
                        }

                        // Standard handling for functions
                        let func_ty = self.infer_type(func.as_ref());
                        let expected_param_types =
                            if let Type::Func { params, .. } = self.resolve_type(&func_ty) {
                                params
                            } else {
                                // Fallback for top-level functions if infer_type didn't find them
                                if let Some(info) = self
                                    .scope
                                    .lookup(name)
                                    .or_else(|| self.scope.lookup(&self.to_snake_case(name)))
                                {
                                    if let Type::Func { params, .. } = self.resolve_type(&info.ty) {
                                        params
                                    } else {
                                        vec![]
                                    }
                                } else {
                                    vec![]
                                }
                            };
                        let ir_args = self.analyze_call_args(
                            &resolved_args,
                            &expected_param_types,
                            &self.get_func_name_for_debug(func.as_ref()),
                        )?;

                        let final_func = if name == "main" {
                            Box::new(self.create_expr(IrExprKind::Var("main_py".to_string()), Type::Unknown))
                        } else {
                            Box::new(self.create_expr(IrExprKind::Var(name.clone()), Type::Unknown))
                        };

                        // V1.5.2: Check if callee may raise (from scope, set by forward_declare_functions)
                        let callee_may_raise = if let Some(var_info) = self.scope.lookup(name) {
                            matches!(
                                &var_info.ty,
                                Type::Func {
                                    may_raise: true,
                                    ..
                                }
                            )
                        } else {
                            // Fallback to may_raise_funcs for functions set during analyze
                            self.may_raise_funcs.contains(name)
                        };

                        // Propagate may_raise to current function
                        if callee_may_raise {
                            self.current_func_may_raise = true;
                        }

                        // V1.7.0: Check if callee needs py_bridge
                        let callee_needs_bridge = self.needs_bridge_funcs.contains(name);

                        // Propagate needs_bridge to current function
                        if callee_needs_bridge {
                            self.current_func_needs_bridge = true;
                        }

                        Ok(self.create_expr(IrExprKind::Call {
                            func: final_func,
                            args: ir_args,
                            callee_may_raise,
                            callee_needs_bridge,
                        }, func_ty))
                    }
                    Expr::Attribute { value, attr } => {
                        // Strip dunder prefix for private fields/methods
                        let stripped_attr = if attr.starts_with("__") && !attr.ends_with("__") {
                            attr.trim_start_matches("__")
                        } else {
                            attr.as_str()
                        };
                        let method_name = match stripped_attr {
                            "append" => "push",
                            other => other,
                        };
                        let ir_target = self.analyze_expr(value)?;
                        let target_ty = self.infer_type(value);

                        // Check if this is a Callable field access (e.g., self.condition_function)
                        // In Rust, calling a field that is a function requires (self.field)(args) syntax
                        if let Expr::Ident(target_name) = value.as_ref() {
                            if target_name == "self" {
                                // Look up the field type
                                let field_lookup = format!("self.{stripped_attr}");
                                let field_type_info = if let Some(info) = self.scope.lookup(&field_lookup) {
                                    Some(info.ty.clone())
                                } else {
                                    None
                                };
                                
                                if let Some(field_ty) = field_type_info {
                                    // Resolve type aliases (e.g., ConditionFunction -> Func)
                                    let resolved_ty = self.resolve_type(&field_ty);
                                    if let Type::Func { .. } = resolved_ty {
                                        // This is a Callable field - emit as Call not MethodCall
                                        let ir_args =
                                            self.analyze_call_args(args, &[], &field_lookup)?;
                                        let field_access = self.create_expr(IrExprKind::FieldAccess {
                                            target: Box::new(ir_target),
                                            field: stripped_attr.to_string(),
                                        }, field_ty);
                                        return Ok(self.create_expr(IrExprKind::Call {
                                            func: Box::new(field_access),
                                            args: ir_args,
                                            callee_may_raise: false,
                                            callee_needs_bridge: false,
                                        }, Type::Unknown));
                                    }
                                }
                            }
                        }

                        // Try special method handling first
                        if let Some(ir) = self.try_handle_special_method(
                            &ir_target,
                            &target_ty,
                            method_name,
                            args,
                        )? {
                            return Ok(ir);
                        }

                        // Default handling: analyze args and create method call
                        let expected_param_types =
                            self.get_method_param_types(&target_ty, method_name);
                        let ir_args = self.analyze_call_args(
                            args,
                            &expected_param_types,
                            &format!("{}.{}", target_ty.to_rust_string(), method_name),
                        )?;

                        if matches!(target_ty, Type::Any) {
                            // V1.7.0: Remote Method Call via Bridge (with kwargs support)
                            self.current_func_may_raise = true;
                            self.current_func_needs_bridge = true;

                            // Analyze keyword args
                            let mut ir_kwargs = Vec::new();
                            for (k, v) in kwargs {
                                let ir_v = self.analyze_expr(v)?;
                                let arg_ty = self.infer_type(v);
                                let wrapped_ir = if matches!(arg_ty, Type::Any) {
                                    self.create_expr(IrExprKind::Ref(Box::new(ir_v)), Type::Any)
                                } else {
                                    let tnk_val = self.create_expr(IrExprKind::TnkValueFrom(Box::new(ir_v)), Type::Any);
                                    self.create_expr(IrExprKind::Ref(Box::new(tnk_val)), Type::Any)
                                };
                                ir_kwargs.push((k.clone(), wrapped_ir));
                            }

                            return Ok(self.create_expr(IrExprKind::BridgeMethodCall {
                                target: Box::new(ir_target),
                                method: method_name.to_string(),
                                args: ir_args,
                                keywords: ir_kwargs,
                            }, Type::Any));
                        }

                        // V1.5.2: Also check if method may raise
                        let callee_may_raise = match &target_ty {
                            Type::Struct(name) => self.may_raise_funcs.contains(&format!("{}.{}", name, method_name)),
                            _ => false,
                        };

                        // V1.7.0: Also check if method needs bridge
                        let callee_needs_bridge = match &target_ty {
                            Type::Struct(name) => self.needs_bridge_funcs.contains(&format!("{}.{}", name, method_name)),
                            _ => false,
                        };

                        if callee_may_raise {
                            self.current_func_may_raise = true;
                        }

                        if callee_needs_bridge {
                            self.current_func_needs_bridge = true;
                        }

                        Ok(self.create_expr(IrExprKind::MethodCall {
                            target_type: target_ty.clone(),
                            target: Box::new(ir_target),
                            method: method_name.to_string(),
                            args: ir_args,
                            callee_needs_bridge,
                        }, self.infer_type(expr)))
                    }
                    _ => {
                        let func_ty = self.infer_type(func.as_ref());
                        let expected_param_types =
                            if let Type::Func { params, .. } = self.resolve_type(&func_ty) {
                                params
                            } else {
                                vec![]
                            };
                        let ir_args = self.analyze_call_args(
                            args,
                            &expected_param_types,
                            &self.get_func_name_for_debug(func.as_ref()),
                        )?;

                        // V1.5.2: Check if callee may raise (from function type)
                        let callee_ty = self.infer_type(func);
                        let mut callee_may_raise = match &callee_ty {
                            Type::Func { may_raise, .. } => *may_raise,
                            _ => false,
                        };

                        // V1.5.2 (2-Pass): Also check scope directly for user-defined functions
                        // This ensures we get the may_raise status from forward_declare_functions
                        if !callee_may_raise {
                            if let Expr::Ident(func_name) = func.as_ref() {
                                if let Some(var_info) = self.scope.lookup(func_name) {
                                    if let Type::Func {
                                        may_raise: true, ..
                                    } = &var_info.ty
                                    {
                                        callee_may_raise = true;
                                    }
                                }
                            }
                        }

                        // Phase G: from-import functions always may raise
                        if let Expr::Ident(func_name) = func.as_ref() {
                            let is_from_import = self
                                .external_imports
                                .iter()
                                .any(|(_, item)| item == func_name);
                            if is_from_import {
                                callee_may_raise = true;
                            }
                        }

                        // Propagate may_raise to current function
                        if callee_may_raise {
                            self.current_func_may_raise = true;
                        }

                        // V1.7.0: Check if callee needs py_bridge
                        let mut callee_needs_bridge = false;
                        if let Expr::Ident(func_name) = func.as_ref() {
                            callee_needs_bridge = self.needs_bridge_funcs.contains(func_name);
                        }

                        // Propagate needs_bridge to current function
                        if callee_needs_bridge {
                            self.current_func_needs_bridge = true;
                        }

                        let ir_func = self.analyze_expr(func)?;
                        Ok(self.create_expr(IrExprKind::Call {
                            func: Box::new(ir_func),
                            args: ir_args,
                            callee_may_raise,
                            callee_needs_bridge,
                        }, func_ty))
                    }
                }
            }
            Expr::List(elements) => {
                let mut ir_elements = Vec::new();
                for e in elements {
                    ir_elements.push(self.analyze_expr(e)?);
                }
                let elem_type = if let Some(first) = elements.first() {
                    self.infer_type(first)
                } else {
                    Type::Unknown
                };
                Ok(self.create_expr(IrExprKind::List {
                    elem_type: elem_type.clone(),
                    elements: ir_elements,
                }, Type::List(Box::new(elem_type))))
            }
            Expr::Index { target, index } => {
                let ir_target = self.analyze_expr(target)?;
                let ir_index = self.analyze_expr(index)?;

                // For sequence indexing, ensure the index is cast to usize
                let target_ty = self.infer_type(target);
                if matches!(target_ty, Type::Any) {
                    // V1.7.0: Remote Item Access via Bridge
                    self.current_func_may_raise = true;

                    return Ok(self.create_expr(IrExprKind::BridgeItemAccess {
                        target: Box::new(ir_target),
                        index: Box::new(ir_index),
                    }, Type::Any));
                }

                let mut current_target_ty = target_ty.clone();
                while let Type::Ref(inner) | Type::MutRef(inner) = current_target_ty {
                    current_target_ty = *inner;
                }

                let final_index = match current_target_ty {
                    Type::List(_) | Type::Tuple(_) | Type::String => self.create_expr(IrExprKind::Cast {
                        target: Box::new(ir_index),
                        ty: "usize".to_string(),
                    }, Type::Unknown),
                    _ => ir_index,
                };

                Ok(self.create_expr(IrExprKind::Index {
                    target: Box::new(ir_target),
                    index: Box::new(final_index),
                }, self.infer_type(expr)))
            }
            Expr::ListComp {
                elt,
                target,
                iter,
                condition,
            }
            | Expr::GenExpr {
                elt,
                target,
                iter,
                condition,
            } => {
                // Special handling for items() in generator expressions
                // Struct type: call items() directly
                // Dict type: use iter().map(|(k,v)|(*k,v.clone())) for owned values in filter
                let ir_iter = if let Expr::Call {
                    func,
                    args: call_args,
                    ..
                } = iter.as_ref()
                {
                    if call_args.is_empty() {
                        if let Expr::Attribute {
                            value: item_target,
                            attr,
                        } = func.as_ref()
                        {
                            if attr == "items" {
                                let target_ty = self.infer_type(item_target);
                                let ir_target = self.analyze_expr(item_target)?;

                                match target_ty {
                                    Type::Struct(_) => {
                                        // Struct with items() method - call items() directly
                                        self.create_expr(IrExprKind::MethodCall {
                                            target_type: Type::Unknown,
                                            target: Box::new(ir_target),
                                            method: "items".to_string(),
                                            args: vec![],
                                            callee_needs_bridge: false,
                                        }, Type::Unknown)
                                    }
                                    Type::Dict(_, _) => {
                                        // d.iter().map(|(k, v)| (*k, v.clone()))
                                        let iter_call = self.create_expr(IrExprKind::MethodCall {
                                            target_type: Type::Unknown,
                                            target: Box::new(ir_target),
                                            method: "iter".to_string(),
                                            args: vec![],
                                            callee_needs_bridge: false,
                                        }, Type::Unknown);
                                        let raw_code = self.create_expr(IrExprKind::RawCode(
                                            "|(k, v)| (*k, v.clone())".to_string(),
                                        ), Type::Unknown);
                                        self.create_expr(IrExprKind::MethodCall {
                                            target_type: Type::Unknown,
                                            target: Box::new(iter_call),
                                            method: "map".to_string(),
                                            args: vec![raw_code],
                                            callee_needs_bridge: false,
                                        }, Type::Unknown)
                                    }
                                    _ => self.analyze_expr(iter)?,
                                }
                            } else {
                                self.analyze_expr(iter)?
                            }
                        } else {
                            self.analyze_expr(iter)?
                        }
                    } else {
                        self.analyze_expr(iter)?
                    }
                } else {
                    self.analyze_expr(iter)?
                };

                let mut iter_ty = self.infer_type(iter);
                while let Type::Ref(inner) = iter_ty {
                    iter_ty = *inner;
                }

                self.scope.push();

                // Define loop variables using unified helper
                self.define_loop_variables(target, &iter_ty, true);

                let ir_elt = self.analyze_expr(elt)?;
                let ir_condition = if let Some(cond) = condition {
                    // Note: We use into_iter() for items() results, so filter() receives owned values
                    // No need to shadow variables with Ref layer anymore
                    let ir = self.analyze_expr(cond)?;
                    Some(Box::new(ir))
                } else {
                    None
                };
                self.scope.pop();

                Ok(self.create_expr(IrExprKind::ListComp {
                    elt: Box::new(ir_elt),
                    target: target.clone(),
                    iter: Box::new(ir_iter),
                    condition: ir_condition,
                }, self.infer_type(expr)))
            }
            // V1.3.0: Dict comprehension {k: v for target in iter if condition}
            Expr::DictComp {
                key,
                value,
                target,
                iter,
                condition,
            } => {
                // V1.3.0: Handle zip/enumerate in dict comprehension
                // Check if iter is a Call to zip/enumerate
                let (ir_iter, iter_ty) = if let Expr::Call { func, args, kwargs } = iter.as_ref() {
                    if let Expr::Ident(func_name) = func.as_ref() {
                        if func_name == "zip" && args.len() >= 2 && kwargs.is_empty() {
                            // zip(a, b) -> a.iter().zip(b.iter()).map(|(x, y)| (x.clone(), y.clone()))
                            let ir_first = self.analyze_expr(&args[0])?;
                            let ir_second = self.analyze_expr(&args[1])?;

                            let iter_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_first),
                                method: "iter".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                            let second_iter_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_second),
                                method: "iter".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                            let zip_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(iter_call),
                                method: "zip".to_string(),
                                args: vec![second_iter_call],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                            let raw_code = self.create_expr(IrExprKind::RawCode(
                                "|(x, y)| (x.clone(), y.clone())".to_string(),
                            ), Type::Unknown);
                            let mapped = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(zip_call),
                                method: "map".to_string(),
                                args: vec![raw_code],
                                callee_needs_bridge: false,
                            }, Type::Unknown);

                            // Infer element types
                            let first_ty = self.infer_type(&args[0]);
                            let second_ty = self.infer_type(&args[1]);
                            let elem1 = match first_ty {
                                Type::List(e) => *e,
                                _ => Type::Unknown,
                            };
                            let elem2 = match second_ty {
                                Type::List(e) => *e,
                                _ => Type::Unknown,
                            };
                            (mapped, Type::Tuple(vec![elem1, elem2]))
                        } else if func_name == "enumerate" && !args.is_empty() && kwargs.is_empty()
                        {
                            // enumerate(items) -> items.iter().enumerate().map(|(i, x)| (i as i64, x.clone()))
                            let ir_items = self.analyze_expr(&args[0])?;
                            let iter_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_items),
                                method: "iter".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                            let enum_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(iter_call),
                                method: "enumerate".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                            let raw_code = self.create_expr(IrExprKind::RawCode(
                                "|(i, x)| (i as i64, x.clone())".to_string(),
                            ), Type::Unknown);
                            let mapped = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(enum_call),
                                method: "map".to_string(),
                                args: vec![raw_code],
                                callee_needs_bridge: false,
                            }, Type::Unknown);

                            let items_ty = self.infer_type(&args[0]);
                            let elem = match items_ty {
                                Type::List(e) => *e,
                                _ => Type::Unknown,
                            };
                            (mapped, Type::Tuple(vec![Type::Int, elem]))
                        } else {
                            // Default case
                            let ir = self.analyze_expr(iter)?;
                            let ty = self.infer_type(iter);
                            (ir, ty)
                        }
                    } else {
                        let ir = self.analyze_expr(iter)?;
                        let ty = self.infer_type(iter);
                        (ir, ty)
                    }
                } else {
                    let ir = self.analyze_expr(iter)?;
                    let ty = self.infer_type(iter);
                    (ir, ty)
                };

                let mut unwrapped_ty = iter_ty.clone();
                while let Type::Ref(inner) = unwrapped_ty {
                    unwrapped_ty = *inner;
                }

                self.scope.push();
                self.define_loop_variables(target, &unwrapped_ty, true);

                let ir_key = self.analyze_expr(key)?;
                let ir_value = self.analyze_expr(value)?;
                let ir_condition = if let Some(cond) = condition {
                    Some(Box::new(self.analyze_expr(cond)?))
                } else {
                    None
                };
                self.scope.pop();

                Ok(self.create_expr(IrExprKind::DictComp {
                    key: Box::new(ir_key),
                    value: Box::new(ir_value),
                    target: target.clone(),
                    iter: Box::new(ir_iter),
                    condition: ir_condition,
                }, iter_ty.clone())) // Roughly correct, actual will be Dict type
            }
            // V1.6.0: Set comprehension {x for target in iter if condition}
            Expr::SetComp {
                elt,
                target,
                iter,
                condition,
            } => {
                let ir_iter = self.analyze_expr(iter)?;
                let mut iter_ty = self.infer_type(iter);
                while let Type::Ref(inner) = iter_ty {
                    iter_ty = *inner;
                }

                self.scope.push();
                self.define_loop_variables(target, &iter_ty, true);

                let ir_elt = self.analyze_expr(elt)?;
                let ir_condition = if let Some(cond) = condition {
                    Some(Box::new(self.analyze_expr(cond)?))
                } else {
                    None
                };
                self.scope.pop();

                Ok(self.create_expr(IrExprKind::SetComp {
                    elt: Box::new(ir_elt),
                    target: target.clone(),
                    iter: Box::new(ir_iter),
                    condition: ir_condition,
                }, self.infer_type(expr)))
            }
            Expr::IfExp { test, body, orelse } => {
                // V1.5.0: If test is an Optional variable used as condition, convert to is_some()
                let test_ty = self.infer_type(test);
                let is_optional_test = matches!(test_ty, Type::Optional(_));

                // V1.5.0: Also check if test is "x is not None" for Optional x
                let optional_var_in_test = if let Expr::BinOp { left, op, right } = test.as_ref() {
                    if matches!(op, AstBinOp::IsNot) && matches!(right.as_ref(), Expr::NoneLiteral)
                    {
                        if let Expr::Ident(var_name) = left.as_ref() {
                            if matches!(self.infer_type(left), Type::Optional(_)) {
                                Some(var_name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let ir_test = if is_optional_test {
                    // Optional variable as condition -> x.is_some()
                    let inner = self.analyze_expr(test)?;
                    self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(inner),
                        method: "is_some".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Bool)
                } else if matches!(test_ty, Type::List(_)) {
                    // List variable as condition -> !x.is_empty()
                    let inner = self.analyze_expr(test)?;
                    let is_empty_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(inner),
                        method: "is_empty".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Bool);
                    self.create_expr(IrExprKind::UnaryOp {
                        op: IrUnaryOp::Not,
                        operand: Box::new(is_empty_call),
                    }, Type::Bool)
                } else {
                    self.analyze_expr(test)?
                };
                let mut ir_body = self.analyze_expr(body)?;
                let mut ir_orelse = self.analyze_expr(orelse)?;

                // V1.5.0: If body is same Optional var as test, unwrap it
                if is_optional_test {
                    if let (Expr::Ident(test_var), Expr::Ident(body_var)) =
                        (test.as_ref(), body.as_ref())
                    {
                        if test_var == body_var {
                            ir_body = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_body),
                                method: "unwrap".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                        }
                    }
                    // Also if body is Optional type, unwrap it
                    if matches!(self.infer_type(body), Type::Optional(_)) {
                        ir_body = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(ir_body),
                            method: "unwrap".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                    }
                }

                // V1.5.0: If test was "x is not None" and body is x, unwrap body
                if let Some(ref opt_var) = optional_var_in_test {
                    if let Expr::Ident(body_var) = body.as_ref() {
                        if body_var == opt_var {
                            ir_body = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_body),
                                method: "unwrap".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                        }
                    }
                }

                // V1.5.0: If orelse is StringLit, add to_string()
                if matches!(ir_orelse.kind, IrExprKind::StringLit(_)) {
                    ir_orelse = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_orelse),
                        method: "to_string".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::String);
                }
                // V1.5.0: If body is StringLit, add to_string()
                if matches!(ir_body.kind, IrExprKind::StringLit(_)) {
                    ir_body = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_body),
                        method: "to_string".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::String);
                }

                // V1.5.0: Optional test + len(x) -> len(x.unwrap())
                if is_optional_test {
                    if let Expr::Ident(test_var) = test.as_ref() {
                        if let IrExprKind::BuiltinCall { id: BuiltinId::Len, args } = &ir_body.kind {
                            if args.len() == 1 {
                                if let IrExprKind::Var(arg_name) = &args[0].kind {
                                    if arg_name == test_var {
                                        let unwrapped = self.create_expr(IrExprKind::Unwrap(Box::new(args[0].clone())), Type::Unknown);
                                        ir_body = self.create_expr(IrExprKind::BuiltinCall {
                                            id: BuiltinId::Len,
                                            args: vec![unwrapped],
                                        }, Type::Int);
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(self.create_expr(IrExprKind::IfExp {
                    test: Box::new(ir_test),
                    body: Box::new(ir_body),
                    orelse: Box::new(ir_orelse),
                }, self.infer_type(expr)))
            }
            Expr::Tuple(elements) => {
                let mut ir_elements = Vec::new();
                for e in elements {
                    ir_elements.push(self.analyze_expr(e)?);
                }
                Ok(self.create_expr(IrExprKind::Tuple(ir_elements), self.infer_type(expr)))
            }
            Expr::Dict(entries) => {
                let mut ir_entries = Vec::new();
                for (k, v) in entries {
                    let ir_key = self.analyze_expr(k)?;
                    let ir_value = self.analyze_expr(v)?;
                    let val_ty = self.infer_type(v);

                    // Auto-convert string literals in Dict to String (.to_string())
                    let final_val = if let Type::String = val_ty {
                        if let IrExprKind::StringLit(_) = ir_value.kind {
                            self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_value),
                                method: "to_string".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::String)
                        } else {
                            ir_value
                        }
                    } else {
                        ir_value
                    };

                    ir_entries.push((ir_key, final_val));
                }
                let mut final_key_type = Type::Unknown;
                let mut final_value_type = Type::Unknown;

                if let Some((first_k, first_v)) = entries.first() {
                    final_key_type = self.infer_type(first_k);
                    final_value_type = self.infer_type(first_v);

                    for (k, v) in entries.iter().skip(1) {
                        let kt = self.infer_type(k);
                        let vt = self.infer_type(v);

                        if kt != final_key_type {
                            final_key_type = Type::Any;
                        }
                        if vt != final_value_type {
                            final_value_type = Type::Any;
                        }
                    }
                }

                Ok(self.create_expr(IrExprKind::Dict {
                    key_type: final_key_type.clone(),
                    value_type: final_value_type.clone(),
                    entries: ir_entries,
                }, Type::Dict(Box::new(final_key_type), Box::new(final_value_type))))
            }
            // V1.5.0: Set literal
            Expr::Set(elements) => {
                let mut ir_elements = Vec::new();
                let mut elem_type = Type::Unknown;

                for (i, e) in elements.iter().enumerate() {
                    let ir_elem = self.analyze_expr(e)?;
                    ir_elements.push(ir_elem);

                    let et = self.infer_type(e);
                    if i == 0 {
                        elem_type = et;
                    } else if et != elem_type {
                        elem_type = Type::Any;
                    }
                }

                Ok(self.create_expr(IrExprKind::Set {
                    elem_type: elem_type.clone(),
                    elements: ir_elements,
                }, Type::Set(Box::new(elem_type.clone()))))
            }
            Expr::FString { parts, values } => {
                let mut ir_values = Vec::new();
                for v in values {
                    let raw_ty = self.infer_type(v);
                    let ty = self.resolve_type(&raw_ty);
                    let ir = self.analyze_expr(v)?;
                    ir_values.push((ir, ty));
                }
                Ok(self.create_expr(IrExprKind::FString {
                    parts: parts.clone(),
                    values: ir_values,
                }, Type::String))
            }
            Expr::Lambda { params, body } => {
                // Convert lambda to closure (lambda x, y: x + y -> move |x, y| x + y)
                // Push a new scope for parameters
                self.scope.push();

                // Define parameters in scope (with Unknown type for now)
                for param in params {
                    self.scope.define(param, Type::Unknown, false);
                }

                // Analyze body expression
                let ir_body = self.analyze_expr(body)?;

                self.scope.pop();

                let ret_type = self.infer_type(body);
                let closure_expr = self.create_expr(IrExprKind::Closure {
                    params: params.clone(),
                    body: vec![IrNode::Return(Some(Box::new(ir_body)))],
                    ret_type: ret_type.clone(),
                }, Type::Func {
                    params: vec![Type::Unknown; params.len()], // Placeholder
                    ret: Box::new(ret_type),
                    may_raise: false,
                    is_boxed: true,
                });
                Ok(self.create_expr(IrExprKind::BoxNew(Box::new(closure_expr)), Type::Func {
                    params: vec![Type::Unknown; params.len()], // Placeholder
                    ret: Box::new(Type::Unknown), // Placeholder
                    may_raise: false,
                    is_boxed: true,
                }))
            }
            Expr::Starred(inner) => {
                // Starred expression (*expr) - analyze the inner expression
                // The caller context will determine how to handle the spread
                let ir_inner = self.analyze_expr(inner)?;
                // For now, just return the inner expression - the context handles spread
                Ok(ir_inner)
            }
            Expr::Slice {
                target,
                start,
                end,
                step,
            } => {
                // Python slices: nums[:3], nums[-3:], nums[1:len(nums)-1], nums[::2], nums[::-1]
                // Rust equivalents depend on the slice type
                let ir_target = self.analyze_expr(target)?;
                let target_type = self.infer_type(target);

                // V1.7.0: Bridge Slice for Any type (Remote Handle)
                if matches!(target_type, Type::Any) {
                    self.current_func_may_raise = true;
                    // Handle start/end/step options, default to NoneLit
                    let ir_start = match start {
                        Some(s) => Box::new(self.analyze_expr(s)?),
                        None => Box::new(self.create_expr(IrExprKind::NoneLit, Type::Optional(Box::new(Type::Unknown)))),
                    };
                    let ir_stop = match end {
                        Some(e) => Box::new(self.analyze_expr(e)?),
                        None => Box::new(self.create_expr(IrExprKind::NoneLit, Type::Optional(Box::new(Type::Unknown)))),
                    };
                    let ir_step = match step {
                        Some(s) => Box::new(self.analyze_expr(s)?),
                        None => Box::new(self.create_expr(IrExprKind::NoneLit, Type::Optional(Box::new(Type::Unknown)))),
                    };

                    return Ok(self.create_expr(IrExprKind::BridgeSlice {
                        target: Box::new(ir_target),
                        start: ir_start,
                        stop: ir_stop,
                        step: ir_step,
                    }, Type::Any));
                }

                let ir_start = match start {
                    Some(s) => Some(Box::new(self.analyze_expr(s)?)),
                    None => None,
                };

                let ir_end = match end {
                    Some(e) => Some(Box::new(self.analyze_expr(e)?)),
                    None => None,
                };

                let ir_step = match step {
                    Some(s) => Some(Box::new(self.analyze_expr(s)?)),
                    None => None,
                };

                // V1.5.0: Special handling for String step slices (use chars() instead of iter())
                if let Some(ref step_box) = ir_step {
                    if matches!(target_type, Type::String) {
                        let target_str = self.emit_simple_ir_expr(&ir_target);
                        let step_val_str = self.emit_simple_ir_expr(step_box);

                        // Check if step is -1 (reverse)
                        let is_reverse = matches!(step_box.kind, IrExprKind::IntLit(-1));

                        if is_reverse {
                            // s[::-1] -> s.chars().rev().collect::<String>()
                            return Ok(self.create_expr(IrExprKind::RawCode(format!(
                                "{}.chars().rev().collect::<String>()",
                                target_str
                            )), Type::String));
                        } else {
                            // s[::n] -> s.chars().step_by(n).collect::<String>()
                            return Ok(self.create_expr(IrExprKind::RawCode(format!(
                                "{}.chars().step_by({} as usize).collect::<String>()",
                                target_str, step_val_str
                            )), Type::String));
                        }
                    }
                }

                Ok(self.create_expr(IrExprKind::Slice {
                    target: Box::new(ir_target),
                    start: ir_start,
                    end: ir_end,
                    step: ir_step,
                }, Type::Unknown)) // TODO: Better slice type inference
            }
            Expr::Attribute { value, attr } => {
                // V1.4.0: Check for native module attribute access (math.pi, etc.)
                if let Expr::Ident(module_name) = value.as_ref() {
                    // Resolve module alias (e.g., m -> math)
                    let real_module = self
                        .module_global_aliases
                        .get(module_name)
                        .map(|s| s.as_str())
                        .unwrap_or(module_name);

                    let full_target = format!("{real_module}.{attr}");
                    if crate::bridge::module_table::is_native_target(&full_target) {
                        if let Some(code) =
                            crate::bridge::module_table::generate_native_code(&full_target, &[])
                        {
                            return Ok(self.create_expr(IrExprKind::RawCode(code), Type::Any));
                        }
                    }
                }

                // V1.6.0: Check for self.field access that should be self.base.field
                if let Expr::Ident(target_name) = value.as_ref() {
                    if target_name == "self" {
                        // Check if this field belongs to parent class
                        if let Some(ref parent) = self.current_class_base {
                            if let Some(parent_fields) = self.struct_field_types.get(parent) {
                                // Strip dunder prefix for comparison
                                let rust_attr = if attr.starts_with("__") && !attr.ends_with("__") {
                                    attr.trim_start_matches("__")
                                } else {
                                    attr.as_str()
                                };
                                // Check if this is a parent field
                                if parent_fields.iter().any(|(f, _)| f == rust_attr) {
                                    // Transform self.field -> self.base.field
                                    let field_ty = parent_fields.iter().find(|(f, _)| f == rust_attr).map(|(_, ty)| ty.clone()).unwrap_or(Type::Unknown);
                                    
                                    let self_var = self.create_expr(IrExprKind::Var("self".to_string()), Type::Unknown);
                                    let base_access = self.create_expr(IrExprKind::FieldAccess {
                                        target: Box::new(self_var),
                                        field: "base".to_string(),
                                    }, Type::Unknown);
                                    
                                    let field_access = self.create_expr(IrExprKind::FieldAccess {
                                        target: Box::new(base_access),
                                        field: rust_attr.to_string(),
                                    }, field_ty);
                                    
                                    return Ok(field_access);
                                }
                            }
                        }
                    }
                }

                // V1.7.0: Remote Attribute Access via Bridge
                let target_ty = self.infer_type(value);
                if matches!(target_ty, Type::Any) {
                    self.current_func_may_raise = true;
                    let target = self.analyze_expr(value)?;
                    return Ok(self.create_expr(IrExprKind::BridgeAttributeAccess {
                        target: Box::new(target),
                        attribute: attr.clone(),
                    }, Type::Any));
                }

                // Standalone attribute access (not call)
                // Could be field access.
                let ir_target = self.analyze_expr(value)?;
                // Strip dunder prefix for Python private fields -> Rust struct field
                let rust_field = if attr.starts_with("__") && !attr.ends_with("__") {
                    attr.trim_start_matches("__").to_string()
                } else {
                    attr.clone()
                };
                Ok(self.create_expr(IrExprKind::FieldAccess {
                    target: Box::new(ir_target),
                    field: rust_field,
                }, Type::Unknown)) // TODO: Better field type inference
            }
            Expr::UnaryOp { op, operand } => {
                let ir_operand = self.analyze_expr(operand)?;
                if let AstUnaryOp::Pos = op {
                    Ok(ir_operand)
                } else {
                    let ir_op = match op {
                        AstUnaryOp::Not => IrUnaryOp::Not,
                        AstUnaryOp::Neg => IrUnaryOp::Neg,
                        AstUnaryOp::Pos => unreachable!(),
                        AstUnaryOp::BitNot => IrUnaryOp::BitNot, // V1.3.0
                    };

                    // V1.6.0 FT-008: not 演算子のオペランドが Type::Any の場合、as_bool().unwrap_or(false) を適用
                    let ir_operand = if matches!(ir_op, IrUnaryOp::Not) {
                        let operand_ty = self.infer_type(operand);
                        if matches!(operand_ty, Type::Any) {
                            let as_bool_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Any,
                                target: Box::new(ir_operand),
                                method: "as_bool".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                            let false_lit = self.create_expr(IrExprKind::BoolLit(false), Type::Bool);
                            self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Any,
                                target: Box::new(as_bool_call),
                                method: "unwrap_or".to_string(),
                                args: vec![false_lit],
                                callee_needs_bridge: false,
                            }, Type::Bool)
                        } else {
                            ir_operand
                        }
                    } else {
                        ir_operand
                    };

                    Ok(self.create_expr(IrExprKind::UnaryOp {
                        op: ir_op,
                        operand: Box::new(ir_operand),
                    }, self.infer_type(expr)))
                }
            }
        }
    }

    fn try_handle_builtin_call(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<Option<IrExpr>, TsuchinokoError> {
        // Special handling for int() and float() casting/conversion
        if args.len() == 1 {
            if name == "int" {
                let arg_ty = self.infer_type(&args[0]);
                let arg = self.analyze_expr(&args[0])?;
                if matches!(arg_ty, Type::Any) {
                    return Ok(Some(self.create_expr(IrExprKind::JsonConversion {
                        target: Box::new(arg),
                        convert_to: "i64".to_string(),
                    }, Type::Int)));
                } else {
                    return Ok(Some(self.create_expr(IrExprKind::Cast {
                        target: Box::new(arg),
                        ty: "i64".to_string(),
                    }, Type::Int)));
                }
            } else if name == "float" {
                let arg_ty = self.infer_type(&args[0]);
                let arg = self.analyze_expr(&args[0])?;
                if matches!(arg_ty, Type::Any) {
                    return Ok(Some(self.create_expr(IrExprKind::JsonConversion {
                        target: Box::new(arg),
                        convert_to: "f64".to_string(),
                    }, Type::Float)));
                } else {
                    return Ok(Some(self.create_expr(IrExprKind::Cast {
                        target: Box::new(arg),
                        ty: "f64".to_string(),
                    }, Type::Float)));
                }
            } else if name == "str" {
                let arg = self.analyze_expr(&args[0])?;
                return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(arg),
                    method: "to_string".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::String)));
            } else if name == "list" {
                // list(iterable) -> iterable.collect::<Vec<_>>()
                // Special case: list(dict.items()) needs iter().map(|(k,v)| (*k, v.clone())).collect()
                if let Expr::Call { func, args: call_args, .. } = &args[0] {
                    if call_args.is_empty() {
                        if let Expr::Attribute { value: item_target, attr } = func.as_ref() {
                            if attr == "items" {
                                let target_ty = self.infer_type(item_target);
                                if matches!(target_ty, Type::Dict(_, _)) {
                                    let ir_target = self.analyze_expr(item_target)?;
                                    let iter_call = self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(ir_target),
                                        method: "iter".to_string(),
                                        args: vec![],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown);
                                    let map_fn = self.create_expr(IrExprKind::RawCode(
                                        "|(k, v)| (*k, v.clone())".to_string(),
                                    ), Type::Unknown);
                                    let map_call = self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(iter_call),
                                        method: "map".to_string(),
                                        args: vec![map_fn],
                                        callee_needs_bridge: false,
                                    }, Type::Unknown);
                                    return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                                        target_type: Type::Unknown,
                                        target: Box::new(map_call),
                                        method: "collect::<Vec<_>>".to_string(),
                                        args: vec![],
                                        callee_needs_bridge: false,
                                    }, Type::Any)));
                                }
                            }
                        }
                    }
                }

                let arg = self.analyze_expr(&args[0])?;
                if matches!(arg.kind, IrExprKind::MethodCall { .. } | IrExprKind::ListComp { .. }) {
                    return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(arg),
                        method: "collect::<Vec<_>>".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Any)));
                }
                return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(arg),
                    method: "to_vec".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Any)));
            } else if name == "tuple" {
                let ir_arg = self.analyze_expr(&args[0])?;
                if matches!(ir_arg.kind, IrExprKind::ListComp { .. } | IrExprKind::MethodCall { .. }) {
                    return Ok(Some(ir_arg));
                }
                return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(ir_arg),
                    method: "collect::<Vec<_>>".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Any)));
            } else if name == "dict" {
                 let ir_arg = self.analyze_expr(&args[0])?;
                 return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                     target_type: Type::Unknown,
                     target: Box::new(ir_arg),
                     method: "clone".to_string(),
                     args: vec![],
                     callee_needs_bridge: false,
                 }, Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown)))));
            } else if name == "max" {
                let arg = self.analyze_expr(&args[0])?;
                let iter_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(arg),
                    method: "iter".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                let max_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(iter_call),
                    method: "max".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                let copied_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(max_call),
                    method: "cloned".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(copied_call),
                    method: "unwrap".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown)));
            } else if name == "min" {
                let arg = self.analyze_expr(&args[0])?;
                let iter_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(arg),
                    method: "iter".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                let min_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(iter_call),
                    method: "min".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                let copied_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(min_call),
                    method: "cloned".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(copied_call),
                    method: "unwrap".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown)));
            } else if name == "sum" {
                let arg = self.analyze_expr(&args[0])?;
                let iter_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(arg),
                    method: "iter".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(iter_call),
                    method: "sum".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown)));
            } else if name == "any" {
                let arg = self.analyze_expr(&args[0])?;
                let iter_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(arg),
                    method: "iter".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                let any_fn = self.create_expr(IrExprKind::RawCode("|x| *x".to_string()), Type::Unknown);
                return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(iter_call),
                    method: "any".to_string(),
                    args: vec![any_fn],
                    callee_needs_bridge: false,
                }, Type::Bool)));
            } else if name == "all" {
                let arg = self.analyze_expr(&args[0])?;
                let iter_call = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(arg),
                    method: "iter".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                let all_fn = self.create_expr(IrExprKind::RawCode("|x| *x".to_string()), Type::Unknown);
                return Ok(Some(self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(iter_call),
                    method: "all".to_string(),
                    args: vec![all_fn],
                    callee_needs_bridge: false,
                }, Type::Bool)));
            }
        }

        // Look up in BuiltinTable
        if let Some(spec) = crate::bridge::builtin_table::get_builtin_spec(name) {
            // Analyze arguments
            let mut ir_args = Vec::new();
            let mut arg_types = Vec::new();
            
            for arg in args {
                let ir_arg = self.analyze_expr(arg)?;
                let actual_ty = self.infer_type(arg);
                
                ir_args.push(ir_arg);
                arg_types.push(actual_ty);
            }

            // Resolve return type using table logic
            let ret_ty = (spec.ret_ty_resolver)(&arg_types);

            // Create IrExprKind::BuiltinCall
            return Ok(Some(self.create_expr(IrExprKind::BuiltinCall {
                id: spec.id,
                args: ir_args,
            }, ret_ty)));
        }

        Ok(None)
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::semantic::analyze;

    #[test]
    fn test_expr_to_type_callable() {
        let analyzer = SemanticAnalyzer::new();
        // Construct Expr for Callable[[int, int], bool]
        // Parser logic simulation:
        // Callable -> Ident
        // [ ... ] -> Index
        // Content is Tuple(List([int, int]), bool)

        let index_expr = Expr::Tuple(vec![
            Expr::List(vec![
                Expr::Ident("int".to_string()),
                Expr::Ident("int".to_string()),
            ]),
            Expr::Ident("bool".to_string()),
        ]);

        let expr = Expr::Index {
            target: Box::new(Expr::Ident("Callable".to_string())),
            index: Box::new(index_expr),
        };

        let ty = analyzer.expr_to_type(&expr);

        if let Some(Type::Func {
            params,
            ret,
            is_boxed,
            ..
        }) = ty
        {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], Type::Int);
            assert_eq!(params[1], Type::Int);
            assert_eq!(*ret, Type::Bool);
            assert!(is_boxed);
        } else {
            panic!("Failed to parse Callable type: {ty:?}");
        }
    }

    // --- analyze_expr テスト ---
    #[test]
    fn test_analyze_expr_int() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::IntLiteral(42);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::IntLit(42)));
    }

    #[test]
    fn test_analyze_expr_float() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::FloatLiteral(3.14);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        if let IrExprKind::FloatLit(f) = ir.kind {
            assert!((f - 3.14).abs() < 0.001);
        }
    }

    #[test]
    fn test_analyze_expr_string() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::StringLiteral("hello".to_string());
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::StringLit(_)));
    }

    #[test]
    fn test_analyze_expr_bool() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BoolLiteral(true);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BoolLit(true)));
    }

    #[test]
    fn test_analyze_expr_none() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::NoneLiteral;
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::NoneLit));
    }

    #[test]
    fn test_analyze_expr_ident() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.define("x", Type::Int, false);
        let expr = Expr::Ident("x".to_string());
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::Var(_)));
    }

    #[test]
    fn test_analyze_expr_list() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::List(vec![Expr::IntLiteral(1), Expr::IntLiteral(2)]);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        if let IrExprKind::List { elements, .. } = ir.kind {
            assert_eq!(elements.len(), 2);
        }
    }

    #[test]
    fn test_analyze_expr_tuple() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::Tuple(vec![Expr::IntLiteral(1), Expr::IntLiteral(2)]);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        if let IrExprKind::Tuple(elements) = ir.kind {
            assert_eq!(elements.len(), 2);
        }
    }

    // --- infer_type テスト ---
    #[test]
    fn test_infer_type_int() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::IntLiteral(42);
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_type_float() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::FloatLiteral(3.14);
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::Float);
    }

    #[test]
    fn test_infer_type_string() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::StringLiteral("hello".to_string());
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_infer_type_bool() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::BoolLiteral(true);
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_type_none() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::NoneLiteral;
        let ty = analyzer.infer_type(&expr);
        // Noneの型推論結果を確認（実装依存）
        // Optional<Unknown>またはUnknownのいずれか
        assert!(matches!(ty, Type::Optional(_) | Type::Unknown));
    }

    #[test]
    fn test_infer_type_list() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::List(vec![Expr::IntLiteral(1)]);
        let ty = analyzer.infer_type(&expr);
        assert!(matches!(ty, Type::List(_)));
    }

    // --- BinOp テスト ---
    #[test]
    fn test_analyze_expr_binop_add() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(1)),
            op: crate::parser::BinOp::Add,
            right: Box::new(Expr::IntLiteral(2)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BinOp { .. }));
    }

    #[test]
    fn test_analyze_expr_binop_sub() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(5)),
            op: crate::parser::BinOp::Sub,
            right: Box::new(Expr::IntLiteral(3)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BinOp { .. }));
    }

    #[test]
    fn test_analyze_expr_binop_mul() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(2)),
            op: crate::parser::BinOp::Mul,
            right: Box::new(Expr::IntLiteral(3)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BinOp { .. }));
    }

    #[test]
    fn test_analyze_expr_binop_div() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(6)),
            op: crate::parser::BinOp::Div,
            right: Box::new(Expr::IntLiteral(2)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BinOp { .. }));
    }

    #[test]
    fn test_analyze_expr_binop_eq() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(1)),
            op: crate::parser::BinOp::Eq,
            right: Box::new(Expr::IntLiteral(1)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BinOp { .. }));
    }

    #[test]
    fn test_analyze_expr_binop_lt() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(1)),
            op: crate::parser::BinOp::Lt,
            right: Box::new(Expr::IntLiteral(2)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BinOp { .. }));
    }

    #[test]
    fn test_analyze_expr_binop_and() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::BoolLiteral(true)),
            op: crate::parser::BinOp::And,
            right: Box::new(Expr::BoolLiteral(false)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BinOp { .. }));
    }

    #[test]
    fn test_analyze_expr_binop_or() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::BoolLiteral(true)),
            op: crate::parser::BinOp::Or,
            right: Box::new(Expr::BoolLiteral(false)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::BinOp { .. }));
    }

    // --- UnaryOp テスト ---
    #[test]
    fn test_analyze_expr_unary_neg() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::UnaryOp {
            op: crate::parser::UnaryOp::Neg,
            operand: Box::new(Expr::IntLiteral(5)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::UnaryOp { .. }));
    }

    #[test]
    fn test_analyze_expr_unary_not() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::UnaryOp {
            op: crate::parser::UnaryOp::Not,
            operand: Box::new(Expr::BoolLiteral(true)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::UnaryOp { .. }));
    }

    // --- Dict テスト ---
    #[test]
    fn test_analyze_expr_dict() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::Dict(vec![(
            Expr::StringLiteral("a".to_string()),
            Expr::IntLiteral(1),
        )]);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::Dict { .. }));
    }

    // --- FString テスト ---
    #[test]
    fn test_analyze_expr_fstring() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.define("x", Type::Int, false);
        let expr = Expr::FString {
            parts: vec!["Value: ".to_string(), "".to_string()],
            values: vec![Expr::Ident("x".to_string())],
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::FString { .. }));
    }

    // --- Index テスト ---
    #[test]
    fn test_analyze_expr_index() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.define("arr", Type::List(Box::new(Type::Int)), false);
        let expr = Expr::Index {
            target: Box::new(Expr::Ident("arr".to_string())),
            index: Box::new(Expr::IntLiteral(0)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::Index { .. }));
    }

    // --- IfExp テスト ---
    #[test]
    fn test_analyze_expr_ifexp() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::IfExp {
            test: Box::new(Expr::BoolLiteral(true)),
            body: Box::new(Expr::IntLiteral(1)),
            orelse: Box::new(Expr::IntLiteral(0)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::IfExp { .. }));
    }

    // --- infer_type 追加テスト ---
    #[test]
    fn test_infer_type_binop() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(1)),
            op: crate::parser::BinOp::Add,
            right: Box::new(Expr::IntLiteral(2)),
        };
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::Int);
    }

    // --- Expr statement ---
    #[test]
    fn test_analyze_expr_stmt() {
        let code = r#"
def test():
    print("hello")
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- infer_type 追加 ---
    #[test]
    fn test_infer_type_unary() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::UnaryOp {
            op: crate::parser::UnaryOp::Neg,
            operand: Box::new(Expr::IntLiteral(5)),
        };
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_type_ifexp() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::IfExp {
            test: Box::new(Expr::BoolLiteral(true)),
            body: Box::new(Expr::IntLiteral(1)),
            orelse: Box::new(Expr::IntLiteral(0)),
        };
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::Int);
    }

    // --- infer from expression ---
    #[test]
    fn test_infer_type_from_literal() {
        let analyzer = SemanticAnalyzer::new();

        assert_eq!(analyzer.infer_type(&Expr::IntLiteral(42)), Type::Int);
        assert_eq!(analyzer.infer_type(&Expr::FloatLiteral(3.14)), Type::Float);
        assert_eq!(
            analyzer.infer_type(&Expr::StringLiteral("test".to_string())),
            Type::String
        );
        assert_eq!(analyzer.infer_type(&Expr::BoolLiteral(true)), Type::Bool);
    }

    // --- complex type inference ---
    #[test]
    fn test_infer_type_complex_binop() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::BinOp {
                left: Box::new(Expr::IntLiteral(1)),
                op: crate::parser::BinOp::Add,
                right: Box::new(Expr::IntLiteral(2)),
            }),
            op: crate::parser::BinOp::Mul,
            right: Box::new(Expr::IntLiteral(3)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Int);
    }

    // --- expression inference ---
    #[test]
    fn test_infer_type_comparison() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(1)),
            op: crate::parser::BinOp::Lt,
            right: Box::new(Expr::IntLiteral(2)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Bool);
    }

    #[test]
    fn test_infer_type_equality() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::BinOp {
            left: Box::new(Expr::IntLiteral(1)),
            op: crate::parser::BinOp::Eq,
            right: Box::new(Expr::IntLiteral(1)),
        };
        assert_eq!(analyzer.infer_type(&expr), Type::Bool);
    }

    // --- more analyze_expressions coverage ---
    #[test]
    fn test_analyze_expr_list_empty() {
        let code = r#"
def test():
    arr = []
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_expr_dict_empty() {
        let code = r#"
def test():
    d = {}
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_expr_parenthesized() {
        let code = r#"
def test():
    x = (1 + 2)
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- infer tests ---
    #[test]
    fn test_infer_type_list_literal() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::List(vec![Expr::IntLiteral(1), Expr::IntLiteral(2)]);
        let ty = analyzer.infer_type(&expr);
        assert!(matches!(ty, Type::List(_)));
    }

    #[test]
    fn test_infer_type_empty_list() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::List(vec![]);
        let ty = analyzer.infer_type(&expr);
        if let Type::List(inner) = ty {
            assert_eq!(*inner, Type::Unknown);
        }
    }

    // --- type inference from variable ---
    #[test]
    fn test_infer_type_from_variable() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.define("x", Type::Int, false);
        let expr = Expr::Ident("x".to_string());
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_type_unknown_variable() {
        let analyzer = SemanticAnalyzer::new();
        let expr = Expr::Ident("unknown".to_string());
        let ty = analyzer.infer_type(&expr);
        assert_eq!(ty, Type::Unknown);
    }

    // --- more expression patterns ---
    #[test]
    fn test_analyze_expr_add() {
        let code = r#"
def test():
    x = 1 + 2
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_expr_sub() {
        let code = r#"
def test():
    x = 5 - 3
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_expr_mul() {
        let code = r#"
def test():
    x = 4 * 5
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_expr_div() {
        let code = r#"
def test():
    x = 10 / 2
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }
}
