//! Statement analysis for SemanticAnalyzer
//!
//! Extracted from mod.rs for maintainability

use super::*;

impl SemanticAnalyzer {
    pub(crate) fn analyze_stmt(&mut self, stmt: &Stmt) -> Result<IrNode, TsuchinokoError> {
        match stmt {
            Stmt::Assign {
                target,
                type_hint,
                value,
            } => {
                // Handle self.field = value pattern
                if target.starts_with("self.") {
                    let field_name = target.trim_start_matches("self.");
                    // Strip dunder prefix for Rust struct field
                    let rust_field_name = field_name.trim_start_matches("__").to_string();
                    let ir_value = self.analyze_expr(value)?;
                    return Ok(IrNode::FieldAssign {
                        target: Box::new(self.create_expr(IrExprKind::Var("self".to_string()), Type::Unknown)),
                        field: rust_field_name,
                        value: Box::new(ir_value),
                    });
                }

                // Check for TypeAlias even in top-level analyze_stmt
                if type_hint.is_none()
                    && target
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false)
                {
                    if let Some(ty) = self.expr_to_type(value) {
                        self.scope.define(target, ty.clone(), false);
                        return Ok(IrNode::TypeAlias {
                            name: target.clone(),
                            ty,
                        });
                    }
                }
                let ty = match type_hint {
                    Some(th) => self.type_from_hint(th),
                    None => self.infer_type(value),
                };

                // Check if variable is already defined (re-assignment)
                let is_reassign = self.scope.lookup(target).is_some();

                // In Python, lists are always mutable. In Rust, we should make them mutable by default
                // to allow modification (like push, index assign).
                // Structs should also be mutable to allow &mut self method calls.
                // Dicts need to be mutable for insert(). Sets for add()/remove().
                // V1.5.0: Also check mutable_vars (collected from pop/update/remove/etc. calls)
                let should_be_mutable = is_reassign
                    || matches!(
                        ty,
                        Type::List(_) | Type::Struct(_) | Type::Dict(_, _) | Type::Set(_)
                    )
                    || self.mutable_vars.contains(target);

                if !is_reassign {
                    self.scope.define(target, ty.clone(), false);
                }

                let ir_value = self.analyze_expr(value)?;

                // If type hint is concrete (String, Int, etc.) but expression is Type::Any,
                // wrap with JsonConversion for proper type conversion
                let expr_ty = self.infer_type(value);
                let ir_value =
                    if matches!(expr_ty, Type::Any) && !matches!(ty, Type::Any | Type::Unknown) {
                        let conversion = match &ty {
                            Type::Float => Some("f64"),
                            Type::Int => Some("i64"),
                            Type::String => Some("String"),
                            Type::Bool => Some("bool"),
                            _ => None,
                        };
                        if let Some(conv) = conversion {
                            self.create_expr(IrExprKind::JsonConversion {
                                target: Box::new(ir_value),
                                convert_to: conv.to_string(),
                            }, ty.clone())
                        } else {
                            ir_value
                        }
                    } else {
                        ir_value
                    };

                // V1.5.0: Wrap non-None values in Some() when assigning to Optional type
                let ir_value = if matches!(ty, Type::Optional(_))
                    && !matches!(value, Expr::NoneLiteral)
                    && !matches!(expr_ty, Type::Optional(_))
                {
                    let some_func = self.create_expr(IrExprKind::Var("Some".to_string()), Type::Unknown);
                    self.create_expr(IrExprKind::Call {
                        func: Box::new(some_func),
                        args: vec![ir_value],
                        callee_may_raise: false,
                        callee_needs_bridge: false,
                    }, ty.clone())
                } else {
                    ir_value
                };


                if is_reassign {
                    Ok(IrNode::Assign {
                        target: target.clone(),
                        value: Box::new(ir_value),
                    })
                } else {
                    // Dead code elimination: If initial value is 0 or empty and type is Int/Float,
                    // and the variable will be shadowed by a for loop, emit a no-op.
                    // However, semantic analysis doesn't have lookahead.
                    // We'll mark this variable as "potentially shadowed" and handle in emit phase.
                    // For now, just emit the declaration.
                    Ok(IrNode::VarDecl {
                        name: target.clone(),
                        ty,
                        mutable: should_be_mutable,
                        init: Some(Box::new(ir_value)),
                    })
                }
            }
            Stmt::IndexAssign {
                target,
                index,
                value,
            } => {
                let target_ty = self.infer_type(target);
                let ir_target = self.analyze_expr(target)?;
                let ir_index = self.analyze_expr(index)?;
                let ir_value = self.analyze_expr(value)?;

                // For sequence indexing, handle Ref/MutRef types
                let mut current_target_ty = target_ty.clone();
                while let Type::Ref(inner) | Type::MutRef(inner) = current_target_ty {
                    current_target_ty = *inner;
                }

                // For Dict types, use insert() method instead of index assignment
                if matches!(current_target_ty, Type::Dict(_, _)) {
                    Ok(IrNode::Expr(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_target),
                        method: "insert".to_string(),
                        args: vec![ir_index, ir_value],
                        callee_needs_bridge: false,
                    }, Type::Unit)))
                } else if matches!(current_target_ty, Type::Any) {
                    // For Any type, use __setitem__ method call
                    Ok(IrNode::Expr(self.create_expr(IrExprKind::PyO3MethodCall {
                        target: Box::new(ir_target),
                        method: "__setitem__".to_string(),
                        args: vec![ir_index, ir_value],
                    }, Type::Any)))
                } else {
                    let final_index = match current_target_ty {
                        Type::List(_) | Type::Tuple(_) | Type::String => {
                            let cast_expr = self.create_expr(IrExprKind::Cast {
                                target: Box::new(ir_index),
                                ty: "usize".to_string(),
                            }, Type::Unknown);
                            cast_expr
                        }
                        _ => ir_index,
                    };

                    Ok(IrNode::IndexAssign {
                        target: Box::new(ir_target),
                        index: Box::new(final_index),
                        value: Box::new(ir_value),
                    })
                }
            }
            Stmt::AugAssign { target, op, value } => {
                // Convert augmented assignment (x += 1) to IR
                let ir_value = self.analyze_expr(value)?;

                // V1.3.0: Special case for String += char (from reversed(str))
                // In Rust, String += char is not allowed, use push() instead
                let target_ty = self
                    .scope
                    .lookup(target)
                    .map(|info| info.ty.clone())
                    .unwrap_or(Type::Unknown);
                if matches!(op, AugAssignOp::Add) && matches!(target_ty, Type::String) {
                    // Convert to: target.push(value)
                    let var_expr = self.create_expr(IrExprKind::Var(target.clone()), target_ty);
                    return Ok(IrNode::Expr(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(var_expr),
                        method: "push".to_string(),
                        args: vec![ir_value],
                        callee_needs_bridge: false,
                    }, Type::Unit)));
                }

                let ir_op = match op {
                    AugAssignOp::Add => IrAugAssignOp::Add,
                    AugAssignOp::Sub => IrAugAssignOp::Sub,
                    AugAssignOp::Mul => IrAugAssignOp::Mul,
                    AugAssignOp::Div => IrAugAssignOp::Div,
                    AugAssignOp::FloorDiv => IrAugAssignOp::FloorDiv,
                    AugAssignOp::Mod => IrAugAssignOp::Mod,
                    // V1.3.0 additions
                    AugAssignOp::Pow => IrAugAssignOp::Pow,
                    AugAssignOp::BitAnd => IrAugAssignOp::BitAnd,
                    AugAssignOp::BitOr => IrAugAssignOp::BitOr,
                    AugAssignOp::BitXor => IrAugAssignOp::BitXor,
                    AugAssignOp::Shl => IrAugAssignOp::Shl,
                    AugAssignOp::Shr => IrAugAssignOp::Shr,
                };
                Ok(IrNode::AugAssign {
                    target: target.clone(),
                    op: ir_op,
                    value: Box::new(ir_value),
                })
            }
            Stmt::TupleAssign {
                targets,
                value,
                starred_index,
            } => {
                // Handle star unpacking: head, *tail = values
                if let Some(star_idx) = starred_index {
                    let ir_value = self.analyze_expr(value)?;
                    let value_ty = self.infer_type(value);

                    // Get the element type from the source value
                    let elem_ty = match &value_ty {
                        Type::List(inner) => *inner.clone(),
                        Type::Ref(inner) => {
                            if let Type::List(elem) = inner.as_ref() {
                                *elem.clone()
                            } else {
                                Type::Unknown
                            }
                        }
                        _ => Type::Unknown,
                    };

                    // Generate individual assignments
                    let mut nodes = Vec::new();

                    for (i, target) in targets.iter().enumerate() {
                        if i == *star_idx {
                            // This is the starred target (e.g., *tail)
                            // Generate: let tail = values[1..].to_vec();
                            let start_idx = i;
                            let end_offset = targets.len() - i - 1;

                            let ty = Type::List(Box::new(elem_ty.clone()));
                            self.scope.define(target, ty.clone(), false);

                            // Build slice expression: values[start_idx..]  or values[start_idx..len-end_offset]
                            let slice_expr = if end_offset == 0 {
                                // values[i..].to_vec()
                            let start_expr = self.create_expr(IrExprKind::IntLit(start_idx as i64), Type::Int);
                            let slice_kind = IrExprKind::Slice {
                                target: Box::new(ir_value.clone()),
                                start: Some(Box::new(start_expr)),
                                end: None,
                                step: None,
                            };
                            let slice_expr = self.create_expr(slice_kind, Type::Unknown);

                            self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(slice_expr),
                                method: "to_vec".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, ty.clone())
                            } else {
                                // values[i..len-end_offset].to_vec()
                                // Need to calculate end index
                                let len_call = self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(ir_value.clone()),
                                    method: "len".to_string(),
                                    args: vec![],
                                    callee_needs_bridge: false,
                                }, Type::Int);
                                let end_offset_expr = self.create_expr(IrExprKind::IntLit(end_offset as i64), Type::Int);
                                let end_expr = self.create_expr(IrExprKind::BinOp {
                                    left: Box::new(len_call),
                                    op: IrBinOp::Sub,
                                    right: Box::new(end_offset_expr),
                                }, Type::Int);
                                let start_expr = self.create_expr(IrExprKind::IntLit(start_idx as i64), Type::Int);
                                let slice_kind = IrExprKind::Slice {
                                    target: Box::new(ir_value.clone()),
                                    start: Some(Box::new(start_expr)),
                                    end: Some(Box::new(end_expr)),
                                    step: None,
                                };
                                let slice_expr = self.create_expr(slice_kind, Type::Unknown);

                                self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(slice_expr),
                                    method: "to_vec".to_string(),
                                    args: vec![],
                                    callee_needs_bridge: false,
                                }, ty.clone())
                            };

                            nodes.push(IrNode::VarDecl {
                                name: target.clone(),
                                ty,
                                mutable: false,
                                init: Some(Box::new(slice_expr)),
                            });
                        } else if i < *star_idx {
                            // Before starred: head = values[i]
                            let ty = elem_ty.clone();
                            self.scope.define(target, ty.clone(), false);

                            let i_expr = self.create_expr(IrExprKind::IntLit(i as i64), Type::Int);
                            let cast_expr = self.create_expr(IrExprKind::Cast {
                                target: Box::new(i_expr),
                                ty: "usize".to_string(),
                            }, Type::Unknown);
                            let index_expr = self.create_expr(IrExprKind::Index {
                                target: Box::new(ir_value.clone()),
                                index: Box::new(cast_expr),
                            }, ty.clone());

                            nodes.push(IrNode::VarDecl {
                                name: target.clone(),
                                ty: elem_ty.clone(),
                                mutable: false,
                                init: Some(Box::new(index_expr)),
                            });
                        } else {
                            // After starred: use negative indexing from end
                            // values[len - (targets.len() - i)]
                            let offset_from_end = targets.len() - i;
                            self.scope.define(target, elem_ty.clone(), false);

                            let len_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_value.clone()),
                                method: "len".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Int);
                            let offset_expr = self.create_expr(IrExprKind::IntLit(offset_from_end as i64), Type::Int);
                            let binop_expr = self.create_expr(IrExprKind::BinOp {
                                left: Box::new(len_call),
                                op: IrBinOp::Sub,
                                right: Box::new(offset_expr),
                            }, Type::Int);
                            let cast_expr = self.create_expr(IrExprKind::Cast {
                                target: Box::new(binop_expr),
                                ty: "usize".to_string(),
                            }, Type::Unknown);

                            let index_expr = self.create_expr(IrExprKind::Index {
                                target: Box::new(ir_value.clone()),
                                index: Box::new(cast_expr),
                            }, elem_ty.clone());

                            nodes.push(IrNode::VarDecl {
                                name: target.clone(),
                                ty: elem_ty.clone(),
                                mutable: false,
                                init: Some(Box::new(index_expr)),
                            });
                        }
                    }

                    return Ok(IrNode::Sequence(nodes));
                }

                // Regular tuple unpacking (no star)
                // Determine if this is a declaration or assignment based on first variable
                // (Simplified logic: if first var is not in scope, assume declaration for all)
                let is_decl = self.scope.lookup(&targets[0]).is_none();
                let ir_value = self.analyze_expr(value)?;

                if is_decl {
                    // Try to infer types if possible, otherwise Unknown
                    // If value is a call, we might not know the return type yet without a better symbol table
                    let result_type = self.infer_type(value);

                    // V1.6.0 FT-008: PyO3 タプルアンパッキング
                    // Type::Any (PyO3 戻り値) の場合は、個別のインデックスアクセスに展開
                    if matches!(result_type, Type::Any) {
                        let mut nodes = Vec::new();

                        // まず一時変数に結果を格納
                        let temp_var = "_tuple_result".to_string();
                        nodes.push(IrNode::VarDecl {
                            name: temp_var.clone(),
                            ty: Type::Any,
                            mutable: false,
                            init: Some(Box::new(ir_value)),
                        });

                        // 各要素をインデックスアクセスで取得
                        for (i, target) in targets.iter().enumerate() {
                            self.scope.define(target, Type::Any, false);
                            let var_expr = self.create_expr(IrExprKind::Var(temp_var.clone()), Type::Any);
                            let i_expr = self.create_expr(IrExprKind::IntLit(i as i64), Type::Int);
                            let cast_expr = self.create_expr(IrExprKind::Cast {
                                target: Box::new(i_expr),
                                ty: "usize".to_string(),
                            }, Type::Unknown);
                            let index_expr = self.create_expr(IrExprKind::Index {
                                target: Box::new(var_expr),
                                index: Box::new(cast_expr),
                            }, Type::Any);

                            let index_access = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Any,
                                target: Box::new(index_expr),
                                method: "clone".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Any);
                            nodes.push(IrNode::VarDecl {
                                name: target.clone(),
                                ty: Type::Any,
                                mutable: false,
                                init: Some(Box::new(index_access)),
                            });
                        }

                        return Ok(IrNode::Sequence(nodes));
                    }

                    let elem_types = if let Type::Tuple(types) = result_type {
                        if types.len() == targets.len() {
                            types
                        } else {
                            vec![Type::Unknown; targets.len()]
                        }
                    } else {
                        vec![Type::Unknown; targets.len()]
                    };

                    let mut decl_targets = Vec::new();
                    for (i, target) in targets.iter().enumerate() {
                        let ty = elem_types.get(i).unwrap_or(&Type::Unknown).clone();
                        self.scope.define(target, ty.clone(), false);
                        decl_targets.push((target.clone(), ty, false));
                    }

                    Ok(IrNode::MultiVarDecl {
                        targets: decl_targets,
                        value: Box::new(ir_value),
                    })
                } else {
                    // Assignment to existing variables
                    Ok(IrNode::MultiAssign {
                        targets: targets.clone(),
                        value: Box::new(ir_value),
                    })
                }
            }
            Stmt::IndexSwap {
                left_targets,
                right_values,
            } => {
                // Handle swap pattern: a[i], a[j] = a[j], a[i]
                // Convert to Rust: a.swap(i as usize, j as usize)
                // This only works for simple 2-element swaps on the same array

                if left_targets.len() == 2 && right_values.len() == 2 {
                    // Check if this is a simple swap pattern:
                    // left_targets[0] matches right_values[1] and left_targets[1] matches right_values[0]
                    if let (
                        Expr::Index {
                            target: t1,
                            index: i1,
                        },
                        Expr::Index {
                            target: t2,
                            index: i2,
                        },
                    ) = (&left_targets[0], &left_targets[1])
                    {
                        // Check if targets are the same array
                        if format!("{t1:?}") == format!("{t2:?}") {
                            // Generate: target.swap(i1 as usize, i2 as usize)
                            let ir_target = self.analyze_expr(t1)?;
                            let ir_i1 = self.analyze_expr(i1)?;
                            let ir_i2 = self.analyze_expr(i2)?;

                            // Cast indices to usize
                            let i1_cast = self.create_expr(IrExprKind::Cast {
                                target: Box::new(ir_i1),
                                ty: "usize".to_string(),
                            }, Type::Unknown);
                            let i2_cast = self.create_expr(IrExprKind::Cast {
                                target: Box::new(ir_i2),
                                ty: "usize".to_string(),
                            }, Type::Unknown);

                            return Ok(IrNode::Expr(self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir_target),
                                method: "swap".to_string(),
                                args: vec![i1_cast, i2_cast],
                                callee_needs_bridge: false,
                            }, Type::Unit)));
                        }
                    }
                }

                // Fallback: not a simple swap, generate temp variable approach
                // For now, return an error for unsupported patterns
                Err(TsuchinokoError::TypeError {
                    line: 0,
                    message: "Unsupported swap pattern - only a[i], a[j] = a[j], a[i] is supported"
                        .to_string(),
                })
            }
            Stmt::FuncDef {
                name,
                params,
                return_type,
                body,
            } => {
                let ret_type = return_type
                    .as_ref()
                    .map(|th| self.type_from_hint(th))
                    .unwrap_or(Type::Unit);

                // Collect mutations in the function body to detect which params need &mut
                let mut reassigned_vars = std::collections::HashSet::new();
                let mut mutated_vars = std::collections::HashSet::new();
                let mut seen_vars = std::collections::HashSet::new();
                for stmt in body {
                    self.collect_mutations(
                        stmt,
                        &mut reassigned_vars,
                        &mut mutated_vars,
                        &mut seen_vars,
                    );
                }

                let mut param_types = Vec::new();

                // V1.5.2: Get refined param types from forward_declare if available
                let refined_func_params: Option<Vec<Type>> =
                    self.scope.lookup(name).and_then(|v| {
                        if let Type::Func { params, .. } = &v.ty {
                            Some(params.clone())
                        } else {
                            None
                        }
                    });

                for (i, p) in params.iter().enumerate() {
                    // First try to use refined type from forward_declare
                    let base_ty = if let Some(ref refined) = refined_func_params {
                        if let Some(refined_ty) = refined.get(i) {
                            // Unwrap Ref/MutRef to get the base type
                            match refined_ty {
                                Type::Ref(inner) | Type::MutRef(inner) => {
                                    // For variadic params, if inner is already List, extract element type
                                    if p.variadic {
                                        if let Type::List(elem_ty) = inner.as_ref() {
                                            elem_ty.as_ref().clone()
                                        } else {
                                            inner.as_ref().clone()
                                        }
                                    } else {
                                        inner.as_ref().clone()
                                    }
                                }
                                // Direct type (no Ref wrapper)
                                _ => {
                                    if p.variadic {
                                        if let Type::List(elem_ty) = refined_ty {
                                            elem_ty.as_ref().clone()
                                        } else {
                                            refined_ty.clone()
                                        }
                                    } else {
                                        refined_ty.clone()
                                    }
                                }
                            }
                        } else {
                            p.type_hint
                                .as_ref()
                                .map(|th| self.type_from_hint(th))
                                .unwrap_or(Type::Unknown)
                        }
                    } else {
                        p.type_hint
                            .as_ref()
                            .map(|th| self.type_from_hint(th))
                            .unwrap_or(Type::Unknown)
                    };

                    // For variadic parameters (*args), wrap in Vec<T>
                    let ty = if p.variadic {
                        Type::List(Box::new(base_ty))
                    // V1.6.0 FT-006: kwargs (**kwargs) は HashMap<String, serde_json::Value>
                    } else if p.is_kwargs {
                        Type::Dict(Box::new(Type::String), Box::new(Type::Any))
                    } else {
                        base_ty
                    };

                    // In Rust, we pass objects by reference.
                    // So if ty is List/Dict/Struct/String/Tuple, the function signature should reflect Ref(ty).
                    // If the parameter is mutated in the function body, use MutRef instead.
                    let is_mutated = mutated_vars.contains(&p.name);
                    let mut signature_ty = match &ty {
                        Type::List(_)
                        | Type::Dict(_, _)
                        | Type::Struct(_)
                        | Type::String
                        | Type::Tuple(_) => {
                            if is_mutated {
                                Type::MutRef(Box::new(ty.clone()))
                            } else {
                                Type::Ref(Box::new(ty.clone()))
                            }
                        }
                        _ => ty.clone(),
                    };

                    // Critical fix for closures: If the function returns a boxed closure,
                    // we need to pass parameters by value (owned) to avoid lifetime issues ('static)
                    if matches!(ret_type, Type::Func { is_boxed: true, .. }) {
                        signature_ty = ty.clone();
                    }

                    param_types.push(signature_ty);
                }

                // If the return type is a Type::Struct (Alias), check if it resolves to a Func.
                // If so, use the Func's return type as the function return type.
                let mut resolved_ret_type = ret_type.clone();
                if let Type::Struct(alias_name) = &ret_type {
                    if let Some(info) = self.scope.lookup(alias_name) {
                        if let Type::Func { ret, .. } = &info.ty {
                            resolved_ret_type = *ret.clone();
                        }
                    }
                }

                // V1.5.2: Preserve may_raise from forward_declare_functions
                let existing_may_raise = self
                    .scope
                    .lookup(name)
                    .and_then(|v| {
                        if let Type::Func { may_raise, .. } = &v.ty {
                            Some(*may_raise)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false);

                // Define function in current scope BEFORE analyzing body (for recursion)
                self.scope.define(
                    name,
                    Type::Func {
                        params: param_types.clone(),
                        ret: Box::new(resolved_ret_type.clone()),
                        is_boxed: false,
                        may_raise: existing_may_raise, // Preserve from forward_declare
                    },
                    false,
                );

                // Register function parameter info for default argument handling at call sites
                let param_info: Vec<(String, Type, Option<Expr>, bool)> = params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let ty = param_types.get(i).cloned().unwrap_or(Type::Unknown);
                        (p.name.clone(), ty, p.default.clone(), p.variadic)
                    })
                    .collect();
                self.func_param_info.insert(name.clone(), param_info);

                // Check if nested function (Closure conversion)
                if self.scope.depth() > 0 {
                    self.scope.push();

                    // Add parameters to scope
                    let mut param_names = Vec::new();

                    for (i, p) in params.iter().enumerate() {
                        let ty = &param_types[i];
                        self.scope.define(&p.name, ty.clone(), false);
                        param_names.push(p.name.clone());
                    }

                    let ir_body = self.analyze_stmts(body)?;
                    self.scope.pop_without_promotion();

                    // Warn about closures if capturing variables?
                    // Currently implicit capture via 'move' in Rust.

                    let closure = self.create_expr(IrExprKind::Closure {
                        params: param_names,
                        body: ir_body,
                        ret_type: ret_type.clone(),
                    }, Type::Func {
                        params: param_types.clone(),
                        ret: Box::new(resolved_ret_type.clone()),
                        is_boxed: false,
                        may_raise: false,
                    });

                    // Wrap closure in Box::new(...) to match Type::Func (Box<dyn Fn...>)
                    let boxed_closure = self.create_expr(IrExprKind::BoxNew(Box::new(closure)), Type::Func {
                        params: param_types.clone(),
                        ret: Box::new(resolved_ret_type.clone()),
                        is_boxed: true,
                        may_raise: false,
                    });

                    return Ok(IrNode::VarDecl {
                        name: name.clone(),
                        ty: Type::Func {
                            params: param_types,
                            ret: Box::new(resolved_ret_type),
                            is_boxed: true,
                            may_raise: false,
                        }, // Variable holding closure is Boxed
                        mutable: false,
                        init: Some(Box::new(boxed_closure)),
                    });
                }

                self.scope.push();

                // Clear hoisted_var_candidates for this function scope
                self.hoisted_var_candidates.clear();
                self.func_base_depth = self.scope.depth();

                // V1.5.2: Save and reset may_raise flag for this function scope
                let old_may_raise = self.current_func_may_raise;
                self.current_func_may_raise = false;
                // V1.7.0: Save and reset needs_bridge flag
                let old_needs_bridge = self.current_func_needs_bridge;
                self.current_func_needs_bridge = false;

                // Add parameters to scope
                let mut ir_params = Vec::new();
                for (i, p) in params.iter().enumerate() {
                    let ty = &param_types[i];
                    self.scope.define(&p.name, ty.clone(), false);
                    ir_params.push((p.name.clone(), ty.clone()));
                }

                // Store the return type for use in Return statement processing
                let old_return_type = self.current_return_type.take();
                self.current_return_type = Some(ret_type.clone());

                let ir_body = self.analyze_stmts(body)?;

                // Restore old return type
                self.current_return_type = old_return_type;

                self.scope.pop_without_promotion();

                // Collect hoisted variables: those used at shallower depth than defined
                let hoisted_vars: Vec<HoistedVar> = self
                    .hoisted_var_candidates
                    .drain()
                    .map(|(name, (ty, _, _))| HoistedVar { name, ty })
                    .collect();

                // V1.5.2: Capture may_raise from this function, then restore parent's flag
                let func_may_raise = self.current_func_may_raise;
                self.current_func_may_raise = old_may_raise;

                // V1.7.0: Capture needs_bridge from this function, then restore parent's flag
                let func_needs_bridge = self.current_func_needs_bridge;
                self.current_func_needs_bridge = old_needs_bridge;

                // V1.5.2: Register function in may_raise_funcs for callee_may_raise detection
                if func_may_raise {
                    self.may_raise_funcs.insert(name.clone());
                }
                // V1.7.0: Register in needs_bridge_funcs
                if func_needs_bridge {
                    self.needs_bridge_funcs.insert(name.clone());
                }

                let ir_name = name.clone();
                Ok(IrNode::FuncDecl {
                    name: ir_name,
                    params: ir_params,
                    ret: ret_type,
                    body: ir_body,
                    hoisted_vars,
                    may_raise: func_may_raise,
                    needs_bridge: func_needs_bridge,
                })
            }

            Stmt::If {
                condition,
                then_body,
                elif_clauses,
                else_body,
            } => {
                // Check for main block
                if let Expr::BinOp { left, op, right } = condition {
                    if let (Expr::Ident(l), AstBinOp::Eq, Expr::StringLiteral(r)) =
                        (left.as_ref(), op, right.as_ref())
                    {
                        if l == "__name__" && r == "__main__" {
                            self.scope.push();
                            let mut ir_body = Vec::new();
                            for s in then_body {
                                ir_body.push(self.analyze_stmt(s)?);
                            }
                            self.scope.pop();

                            return Ok(IrNode::FuncDecl {
                                name: "main".to_string(),
                                params: vec![],
                                ret: Type::Unit,
                                body: ir_body,
                                hoisted_vars: vec![],
                                may_raise: false,
                                needs_bridge: false,
                            });
                        }
                    }
                }

                // V1.6.0 FT-005: Check for isinstance pattern: if isinstance(x, T): ...
                if let Some((var_name, checked_type)) = self.extract_isinstance_check(condition) {
                    // Collect all isinstance arms from if-elif-else chain
                    let mut arms: Vec<crate::ir::nodes::MatchArm> = Vec::new();

                    // First arm from if condition
                    self.scope.push();
                    let variant = self.type_to_dynamic_variant(&checked_type);
                    self.scope.define(&var_name, checked_type.clone(), false);
                    let body: Vec<IrNode> = then_body
                        .iter()
                        .filter_map(|s| self.analyze_stmt(s).ok())
                        .collect();
                    self.scope.pop();

                    arms.push(crate::ir::nodes::MatchArm {
                        variant,
                        binding: var_name.clone(),
                        body,
                    });

                    // Check elif clauses for more isinstance checks
                    for (elif_cond, elif_body) in elif_clauses {
                        if let Some((_elif_var, elif_type)) =
                            self.extract_isinstance_check(elif_cond)
                        {
                            self.scope.push();
                            let variant = self.type_to_dynamic_variant(&elif_type);
                            self.scope.define(&var_name, elif_type, false);
                            let body: Vec<IrNode> = elif_body
                                .iter()
                                .filter_map(|s| self.analyze_stmt(s).ok())
                                .collect();
                            self.scope.pop();

                            arms.push(crate::ir::nodes::MatchArm {
                                variant,
                                binding: var_name.clone(),
                                body,
                            });
                        }
                    }

                    // If there's an else clause, add catch-all arm
                    if let Some(else_stmts) = else_body {
                        self.scope.push();
                        let body: Vec<IrNode> = else_stmts
                            .iter()
                            .filter_map(|s| self.analyze_stmt(s).ok())
                            .collect();
                        self.scope.pop();

                        // Add "other" variant for catch-all
                        arms.push(crate::ir::nodes::MatchArm {
                            variant: "_".to_string(),
                            binding: "other".to_string(),
                            body,
                        });
                    }

                    return Ok(IrNode::Match {
                        value: self.create_expr(IrExprKind::Var(var_name), Type::Any),
                        arms,
                    });
                }

                // Check for type narrowing pattern: `if x is None:` or `if x is not None:`
                // Extract variable name and narrowing direction
                let narrowing_info = self.extract_none_check(condition);

                let ir_cond = self.analyze_expr(condition)?;

                // V1.6.0 FT-008: Type::Any を条件式で使用する場合、as_bool().unwrap_or(false) に変換
                let cond_ty = self.infer_type(condition);
                let ir_cond = if matches!(cond_ty, Type::Any) {
                    let as_bool_expr = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Any,
                        target: Box::new(ir_cond),
                        method: "as_bool".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    let false_lit = self.create_expr(IrExprKind::BoolLit(false), Type::Bool);
                    self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Any,
                        target: Box::new(as_bool_expr),
                        method: "unwrap_or".to_string(),
                        args: vec![false_lit],
                        callee_needs_bridge: false,
                    }, Type::Bool)
                } else {
                    ir_cond
                };

                // Analyze then block with narrowing (if applicable)
                self.scope.push();
                if let Some((var_name, is_none_in_then)) = &narrowing_info {
                    if *is_none_in_then {
                        // In `if x is None:` then block, x is definitely None
                        // (No specific narrowing needed - x stays Optional)
                    } else {
                        // In `if x is not None:` then block, x is definitely NOT None
                        // Narrow to inner type
                        if let Some(Type::Optional(inner)) = self.scope.get_effective_type(var_name)
                        {
                            self.scope.narrow_type(var_name, *inner.clone());
                        }
                    }
                }
                let mut ir_then = Vec::new();
                for s in then_body {
                    ir_then.push(self.analyze_stmt(s)?);
                }
                self.scope.pop();

                // Analyze else block with opposite narrowing
                let mut ir_else = if let Some(else_stmts) = else_body {
                    self.scope.push();
                    if let Some((var_name, is_none_in_then)) = &narrowing_info {
                        if *is_none_in_then {
                            // In `if x is None:` else block, x is definitely NOT None
                            // Narrow to inner type
                            if let Some(Type::Optional(inner)) =
                                self.scope.lookup(var_name).map(|v| v.ty.clone())
                            {
                                self.scope.narrow_type(var_name, *inner);
                            }
                        }
                        // In `if x is not None:` else block, x is None (no narrowing needed)
                    }
                    let mut stmts = Vec::new();
                    for s in else_stmts {
                        stmts.push(self.analyze_stmt(s)?);
                    }
                    self.scope.pop();
                    Some(stmts)
                } else {
                    None
                };

                for (elif_cond, elif_body) in elif_clauses.iter().rev() {
                    let elif_ir_cond = self.analyze_expr(elif_cond)?;
                    self.scope.push();
                    let mut elif_ir_body = Vec::new();
                    for s in elif_body {
                        elif_ir_body.push(self.analyze_stmt(s)?);
                    }
                    self.scope.pop();

                    let nested_if = IrNode::If {
                        cond: Box::new(elif_ir_cond),
                        then_block: elif_ir_body,
                        else_block: ir_else,
                    };
                    ir_else = Some(vec![nested_if]);
                }

                Ok(IrNode::If {
                    cond: Box::new(ir_cond),
                    then_block: ir_then,
                    else_block: ir_else,
                })
            }
            Stmt::For { target, iter, body } => {
                // V1.3.0: Handle enumerate(iterable) and zip(a, b)
                let (actual_target, ir_iter, elem_type) = self.analyze_for_iter(target, iter)?;

                self.scope.push();

                // Define loop variables based on element type
                if actual_target.contains(',') {
                    // Tuple unpacking: i, item OR x, y, z
                    let targets: Vec<_> = actual_target.split(',').map(|s| s.trim()).collect();
                    if let Type::Tuple(types) = &elem_type {
                        for (t, ty) in targets.iter().zip(types.iter()) {
                            self.scope.define(t, ty.clone(), false);
                        }
                    } else {
                        for t in &targets {
                            self.scope.define(t, Type::Unknown, false);
                        }
                    }
                } else {
                    self.scope.define(&actual_target, elem_type.clone(), false);
                }

                // Use analyze_stmts to properly detect mutable variables in loop body
                let ir_body = self.analyze_stmts(body)?;
                self.scope.pop();

                Ok(IrNode::For {
                    var: actual_target,
                    var_type: elem_type,
                    iter: Box::new(ir_iter),
                    body: ir_body,
                })
            }
            Stmt::While { condition, body } => {
                let ir_cond = self.analyze_expr(condition)?;

                self.scope.push();
                // Use analyze_stmts to properly detect mutable variables in loop body
                let ir_body = self.analyze_stmts(body)?;
                self.scope.pop();

                Ok(IrNode::While {
                    cond: Box::new(ir_cond),
                    body: ir_body,
                })
            }
            Stmt::Return(expr) => {
                let ir_expr = match expr {
                    Some(e) => {
                        let ir = self.analyze_expr(e)?;
                        let ty = self.infer_type(e);
                        if let Some(ret_ty) = &self.current_return_type {
                            if !matches!(ret_ty, Type::Any | Type::Unknown) {
                                self.set_type(ir.id, ret_ty.clone());
                            }
                        }

                        // Check if we're returning from an Optional function
                        let is_optional_return =
                            matches!(&self.current_return_type, Some(Type::Optional(_)));

                        // If returning a Reference to a List (slice), use .to_vec() to return owned
                        let ir = if let Type::Ref(inner) = &ty {
                            if matches!(inner.as_ref(), Type::List(_)) {
                                self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(ir),
                                    method: "to_vec".to_string(),
                                    args: vec![],
                                    callee_needs_bridge: false,
                                }, Type::Unknown)
                            } else {
                                self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(ir),
                                    method: "clone".to_string(),
                                    args: vec![],
                                    callee_needs_bridge: false,
                                }, Type::Unknown)
                            }
                        } else {
                            ir
                        };

                        // If returning a string literal to a String return type, add .to_string()
                        let ir = if matches!(self.current_return_type, Some(Type::String))
                            && matches!(ir.kind, IrExprKind::StringLit(_))
                        {
                            self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(ir),
                                method: "to_string".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::String)
                        } else if let Some(Type::Tuple(expected_types_ref)) = &self.current_return_type {
                            let expected_types = expected_types_ref.clone();
                            // Handle tuple return with string elements
                            if let IrExprKind::Tuple(elements) = ir.kind {
                                let converted: Vec<IrExpr> = elements
                                    .into_iter()
                                    .zip(expected_types.iter())
                                    .map(|(elem, expected_ty)| {
                                        if matches!(*expected_ty, Type::String)
                                            && matches!(elem.kind, IrExprKind::StringLit(_))
                                        {
                                            self.create_expr(IrExprKind::MethodCall {
                                                target_type: Type::Unknown,
                                                target: Box::new(elem),
                                                method: "to_string".to_string(),
                                                args: vec![],
                                                callee_needs_bridge: false,
                                            }, Type::String)
                                        } else {
                                            elem
                                        }
                                    })
                                    .collect();
                                self.create_expr(IrExprKind::Tuple(converted), self.current_return_type.clone().unwrap_or(Type::Unknown))
                            } else {
                                ir
                            }
                        } else {
                            ir
                        };

                        // Bridge結果はLoweringでFromTnkValueを挿入するため、OptionalのSomeラップだけ抑止する。
                        let is_bridge_result = matches!(
                            ir.kind,
                            IrExprKind::BridgeCall { .. }
                                | IrExprKind::BridgeMethodCall { .. }
                                | IrExprKind::BridgeGet { .. }
                                | IrExprKind::BridgeAttributeAccess { .. }
                                | IrExprKind::BridgeItemAccess { .. }
                                | IrExprKind::BridgeSlice { .. }
                        ) || matches!(
                            ir.kind,
                            IrExprKind::Call { ref func, .. }
                                if matches!(
                                    func.kind,
                                    IrExprKind::BridgeGet { .. }
                                        | IrExprKind::BridgeAttributeAccess { .. }
                                        | IrExprKind::BridgeItemAccess { .. }
                                        | IrExprKind::BridgeSlice { .. }
                                )
                        );
                        let skip_optional_wrap = matches!(
                            self.current_return_type,
                            Some(Type::Optional(_))
                        ) && is_bridge_result;

                        // Wrap in Some() if returning to Optional and value is not None
                        if is_optional_return && !skip_optional_wrap && !matches!(ir.kind, IrExprKind::NoneLit) {
                            let some_func = self.create_expr(IrExprKind::Var("Some".to_string()), Type::Unknown);
                            Some(Box::new(self.create_expr(IrExprKind::Call {
                                func: Box::new(some_func),
                                args: vec![ir],
                                callee_may_raise: false,
                                callee_needs_bridge: false,
                            }, self.current_return_type.clone().unwrap_or(Type::Unknown))))
                        } else if matches!(ty, Type::Any) && !is_bridge_result {
                            // Convert Type::Any (serde_json::Value) to expected return type
                            if let Some(ret_ty) = &self.current_return_type {
                                let conversion = match ret_ty {
                                    Type::Float => Some("f64"),
                                    Type::Int => Some("i64"),
                                    Type::String => Some("String"),
                                    Type::Bool => Some("bool"),
                                    _ => None,
                                };
                        if let Some(conv) = conversion {
                                    Some(Box::new(self.create_expr(IrExprKind::JsonConversion {
                                        target: Box::new(ir),
                                        convert_to: conv.to_string(),
                                    }, ret_ty.clone())))
                                } else {
                                    Some(Box::new(ir))
                                }
                            } else {
                                Some(Box::new(ir))
                            }
                        } else {
                            Some(Box::new(ir))
                        }
                    }
                    None => None,
                };
                Ok(IrNode::Return(ir_expr))
            }
            Stmt::Expr(expr) => {
                let ir_expr = self.analyze_expr(expr)?;
                Ok(IrNode::Expr(ir_expr))
            }
            Stmt::ClassDef {
                name,
                bases,
                fields,
                methods,
            } => {
                // Convert AST fields to IR fields with types
                let mut ir_fields: Vec<(String, Type)> = fields
                    .iter()
                    .map(|f| {
                        let ty = self.type_from_hint(&f.type_hint);
                        (f.name.clone(), ty)
                    })
                    .collect();

                // V1.6.0: If class has bases, add "base" field for composition
                let base_class = if !bases.is_empty() {
                    // For now, support single inheritance only
                    let parent_name = &bases[0];
                    // Add "base" field of parent type
                    ir_fields.insert(0, ("base".to_string(), Type::Struct(parent_name.clone())));
                    // V1.6.0: Register inheritance relationship
                    self.struct_bases.insert(name.clone(), parent_name.clone());
                    Some(parent_name.clone())
                } else {
                    None
                };

                // Save field types for constructor type checking
                self.struct_field_types
                    .insert(name.clone(), ir_fields.clone());

                // V1.5.2: Save field default values for constructor initialization
                let field_defaults: Vec<(String, IrExpr)> = fields
                    .iter()
                    .filter_map(|f| {
                        f.default_value.as_ref().and_then(|expr| {
                            self.analyze_expr(expr).ok().map(|ir| (f.name.clone(), ir))
                        })
                    })
                    .collect();
                if !field_defaults.is_empty() {
                    self.struct_field_defaults
                        .insert(name.clone(), field_defaults);
                }

                // Register this struct type in scope (for use in type hints)
                self.scope.define(name, Type::Struct(name.clone()), false);

                // If there are methods, create an impl block
                let mut result_nodes = vec![IrNode::StructDef {
                    name: name.clone(),
                    fields: ir_fields.clone(),
                    base: base_class.clone(),
                }];

                if !methods.is_empty() {
                    let mut ir_methods = Vec::new();

                    for method in methods {
                        // Skip __init__ - it's handled via fields
                        if method.name == "__init__" {
                            continue;
                        }

                        // Parse method parameters
                        let ir_params: Vec<(String, Type)> = method
                            .params
                            .iter()
                            .map(|p| {
                                let ty = p
                                    .type_hint
                                    .as_ref()
                                    .map(|h| self.type_from_hint(h))
                                    .unwrap_or(Type::Unknown);
                                (p.name.clone(), ty)
                            })
                            .collect();

                        let ret_ty = method
                            .return_type
                            .as_ref()
                            .map(|h| self.type_from_hint(h))
                            .unwrap_or(Type::Unit);

                        // Analyze method body with self in scope
                        self.scope.push();
                        // V1.6.0: Set current class base for self.field -> self.base.field transform
                        self.current_class_base = base_class.clone();
                        // Define self as this struct type
                        self.scope.define("self", Type::Struct(name.clone()), false);
                        // Define struct fields as self.field_name for type inference
                        for (field_name, field_ty) in &ir_fields {
                            // Strip dunder prefix for consistency
                            let rust_field_name =
                                if field_name.starts_with("__") && !field_name.ends_with("__") {
                                    field_name.trim_start_matches("__")
                                } else {
                                    field_name.as_str()
                                };
                            self.scope.define(
                                &format!("self.{rust_field_name}"),
                                field_ty.clone(),
                                false,
                            );
                        }
                        // V1.6.0: Also register parent fields as self.field (for inheritance)
                        if let Some(ref parent) = base_class {
                            if let Some(parent_fields) =
                                self.struct_field_types.get(parent).cloned()
                            {
                                for (pf_name, pf_ty) in parent_fields {
                                    if pf_name != "base" {
                                        self.scope.define(
                                            &format!("self.{pf_name}"),
                                            pf_ty.clone(),
                                            false,
                                        );
                                    }
                                }
                            }
                        }
                        // Define method params
                        for (p_name, p_ty) in &ir_params {
                            self.scope.define(p_name, p_ty.clone(), false);
                        }

                        // V1.5.2: Save and reset may_raise flag for this method
                        let old_may_raise = self.current_func_may_raise;
                        self.current_func_may_raise = false;
                        // V1.7.0: Save and reset needs_bridge flag
                        let old_needs_bridge = self.current_func_needs_bridge;
                        self.current_func_needs_bridge = false;

                        // FIX: Save and set current_return_type for return value coercion
                        let old_return_type = self.current_return_type.take();
                        self.current_return_type = Some(ret_ty.clone());

                        let ir_body: Vec<IrNode> = method
                            .body
                            .iter()
                            .map(|s| self.analyze_stmt(s))
                            .collect::<Result<Vec<_>, _>>()?;

                        // Capture method's may_raise status and restore state
                        let method_may_raise = self.current_func_may_raise;
                        self.current_func_may_raise = old_may_raise;
                        // V1.7.0: Capture needs_bridge and restore
                        let method_needs_bridge = self.current_func_needs_bridge;
                        self.current_func_needs_bridge = old_needs_bridge;

                        self.current_return_type = old_return_type;

                        self.scope.pop_without_promotion();

                        // Check if method modifies self (contains FieldAssign)
                        let takes_mut_self = ir_body
                            .iter()
                            .any(|node| matches!(node, IrNode::FieldAssign { .. }));

                        // V1.6.0 FT-003: @property setter -> set_xxx メソッドに変換
                        let method_name = if let Some(ref prop_name) = method.setter_for {
                            format!("set_{}", prop_name)
                        } else {
                            method.name.clone()
                        };

                        // V1.6.0 FT-003: setter は &mut self を取る
                        let takes_mut_self = takes_mut_self || method.setter_for.is_some();

                        ir_methods.push(IrNode::MethodDecl {
                            name: method_name.clone(),
                            params: ir_params,
                            ret: ret_ty,
                            body: ir_body,
                            takes_self: !method.is_static,
                            takes_mut_self,
                            may_raise: method_may_raise,
                            needs_bridge: method_needs_bridge,
                        });
                        
                        // Register method in needs_bridge_funcs for call site propagation if it actually needs it
                        if method_needs_bridge {
                            self.needs_bridge_funcs.insert(format!("{}.{}", name, method_name));
                        }
                    }

                    result_nodes.push(IrNode::ImplBlock {
                        struct_name: name.clone(),
                        methods: ir_methods,
                    });
                }

                // Return node(s) - use Sequence if multiple
                if result_nodes.len() == 1 {
                    Ok(result_nodes.remove(0))
                } else {
                    // Return Sequence containing StructDef + ImplBlock
                    Ok(IrNode::Sequence(result_nodes))
                }
            }
            Stmt::Raise {
                exception_type,
                message,
                cause,
                line,
            } => {
                // V1.5.2: Mark current function as may raise
                self.current_func_may_raise = true;

                let msg_ir = self.analyze_expr(message)?;
                // V1.5.2: Analyze cause expression if present
                let cause_ir = match cause {
                    Some(c) => Some(Box::new(self.analyze_expr(c)?)),
                    None => None,
                };
                Ok(IrNode::Raise {
                    exc_type: exception_type.clone(),
                    message: Box::new(msg_ir),
                    cause: cause_ir,
                    line: *line,
                })
            }
            Stmt::TryExcept {
                try_body,
                except_clauses,
                else_body, // V1.5.2: else ブロック
                finally_body,
            } => {
                // Use analyze_stmts to properly detect mutable variables in try/except blocks
                let ir_try_body = self.analyze_stmts(try_body)?;

                // V1.5.2: Collect the first except clause's variable name for IR
                let except_var = except_clauses.iter().find_map(|c| c.name.clone());

                // Collect all except bodies into one (Rust doesn't have typed exceptions)
                // For now, we merge all except clauses into a single except block
                let mut ir_except_body = Vec::new();
                for clause in except_clauses {
                    // If clause has a name (as e), define the variable
                    if let Some(ref name) = clause.name {
                        self.scope.push();
                        self.scope.define(name, Type::String, false);
                    }

                    let clause_body = self.analyze_stmts(&clause.body)?;
                    ir_except_body.extend(clause_body);

                    if clause.name.is_some() {
                        self.scope.pop();
                    }
                }

                // Analyze finally body if present
                let ir_finally_body = if let Some(fb) = finally_body {
                    Some(self.analyze_stmts(fb)?)
                } else {
                    None
                };

                // V1.5.2: Analyze else body if present
                let ir_else_body = if let Some(eb) = else_body {
                    Some(self.analyze_stmts(eb)?)
                } else {
                    None
                };

                Ok(IrNode::TryBlock {
                    try_body: ir_try_body,
                    except_body: ir_except_body,
                    except_var,              // V1.5.2: 例外変数名
                    else_body: ir_else_body, // V1.5.2: else ブロック
                    finally_body: ir_finally_body,
                })
            }
            Stmt::Break => Ok(IrNode::Break),
            Stmt::Continue => Ok(IrNode::Continue),
            // V1.3.0: Assert statement
            Stmt::Assert { test, msg } => {
                let ir_test = self.analyze_expr(test)?;
                let ir_msg = match msg {
                    Some(m) => Some(Box::new(self.analyze_expr(m)?)),
                    None => None,
                };
                Ok(IrNode::Assert {
                    test: Box::new(ir_test),
                    msg: ir_msg,
                })
            }
            Stmt::Import {
                module,
                alias,
                items,
            } => {
                // Triple Hybrid System: Handle native modules via table
                let is_native = crate::bridge::module_table::is_native_module(module);

                if let Some(ref item_list) = items {
                    // "from module import a, b, c"
                    for item in item_list {
                        self.module_global_aliases
                            .insert(item.clone(), format!("{module}.{item}"));
                        if !is_native {
                            self.external_imports.push((module.clone(), item.clone()));
                            self.scope.define(item, Type::Any, false);
                        } else {
                            // Native items like math.pi are resolved via IrExpr mapping
                            self.scope.define(item, Type::Unknown, false);
                        }
                    }
                } else {
                    // "import module" or "import module as alias"
                    let effective_name = alias.as_ref().unwrap_or(module);
                    self.module_global_aliases
                        .insert(effective_name.clone(), module.clone());
                    if !is_native {
                        self.external_imports
                            .push((module.clone(), effective_name.clone()));
                        self.scope.define(effective_name, Type::Any, false);
                    } else {
                        self.scope.define(effective_name, Type::Unknown, false);
                    }
                }

                if is_native {
                    // Native modules (math, etc.) are handled statically, no runtime import needed
                    Ok(IrNode::Sequence(vec![]))
                } else {
                    Ok(IrNode::BridgeImport {
                        module: module.clone(),
                        alias: alias.clone(),
                        items: items.clone(),
                    })
                }
            }
            // V1.6.0: with statement -> scoped block with variable binding
            Stmt::With {
                context_expr,
                optional_vars,
                body,
            } => {
                // V1.6.0: Transform open() calls to File::open/create
                // open("path", "r") -> File::open("path")?
                // open("path", "w") -> File::create("path")?
                let ir_context = self.transform_with_context(context_expr)?;

                // Analyze body statements
                let ir_body: Vec<IrNode> = body
                    .iter()
                    .map(|s| self.analyze_stmt(s))
                    .collect::<Result<Vec<_>, _>>()?;

                // Create a block with optional variable binding
                if let Some(var_name) = optional_vars {
                    // with EXPR as NAME: -> { let NAME = EXPR; BODY }
                    let decl = IrNode::VarDecl {
                        name: var_name.clone(),
                        ty: Type::Unknown, // Type inference will determine
                        init: Some(Box::new(ir_context)),
                        mutable: true, // Files need to be mutable for write operations
                    };

                    let mut block_body = vec![decl];
                    block_body.extend(ir_body);

                    Ok(IrNode::Block { stmts: block_body })
                } else {
                    // with EXPR: -> { EXPR; BODY }
                    let expr_stmt = IrNode::Expr(ir_context);
                    let mut block_body = vec![expr_stmt];
                    block_body.extend(ir_body);

                    Ok(IrNode::Block { stmts: block_body })
                }
            }
        }
    }

    /// V1.6.0: Transform with statement context expression
    /// Converts Python's open() to Rust's File::open/create
    fn transform_with_context(&mut self, expr: &Expr) -> Result<IrExpr, TsuchinokoError> {
        // Check for open() call
        if let Expr::Call {
            func,
            args,
            kwargs: _,
        } = expr
        {
            if let Expr::Ident(func_name) = func.as_ref() {
                if func_name == "open" && !args.is_empty() {
                    // Get the file path
                    let path = self.analyze_expr(&args[0])?;

                    // Determine mode: "r" (default), "w", "a", etc.
                    let mode = if args.len() > 1 {
                        if let Expr::StringLiteral(m) = &args[1] {
                            m.as_str()
                        } else {
                            "r"
                        }
                    } else {
                        "r"
                    };

                    // Transform based on mode
                    let (struct_name, method_name) = match mode {
                        "w" | "w+" | "wb" => ("File", "create"),
                        "a" | "a+" | "ab" => ("File", "options"), // append needs OpenOptions
                        _ => ("File", "open"),                    // "r", "r+", "rb" -> open
                    };

                    // Generate: File::open(path)? or File::create(path)?
                    let raw_code = self.create_expr(IrExprKind::RawCode(format!(
                        "{}::{}",
                        struct_name, method_name
                    )), Type::Unknown);
                    let call_expr = self.create_expr(IrExprKind::Call {
                        func: Box::new(raw_code),
                        args: vec![path],
                        callee_may_raise: true,
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    return Ok(self.create_expr(IrExprKind::Unwrap(Box::new(call_expr)), Type::Unknown));
                }
            }
        }

        // Fallback: just analyze the expression normally
        self.analyze_expr(expr)
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::semantic::analyze;

    fn node_matches(node: &IrNode, pred: &dyn Fn(&IrNode) -> bool) -> bool {
        if pred(node) {
            return true;
        }
        match node {
            IrNode::Sequence(nodes) => nodes.iter().any(|n| node_matches(n, pred)),
            IrNode::Block { stmts } => stmts.iter().any(|n| node_matches(n, pred)),
            IrNode::If { then_block, else_block, .. } => {
                then_block.iter().any(|n| node_matches(n, pred))
                    || else_block
                        .as_ref()
                        .map(|b| b.iter().any(|n| node_matches(n, pred)))
                        .unwrap_or(false)
            }
            IrNode::For { body, .. } => body.iter().any(|n| node_matches(n, pred)),
            IrNode::While { body, .. } => body.iter().any(|n| node_matches(n, pred)),
            IrNode::TryBlock { try_body, except_body, else_body, finally_body, .. } => {
                try_body.iter().any(|n| node_matches(n, pred))
                    || except_body.iter().any(|n| node_matches(n, pred))
                    || else_body
                        .as_ref()
                        .map(|b| b.iter().any(|n| node_matches(n, pred)))
                        .unwrap_or(false)
                    || finally_body
                        .as_ref()
                        .map(|b| b.iter().any(|n| node_matches(n, pred)))
                        .unwrap_or(false)
            }
            IrNode::Match { arms, .. } => arms.iter().any(|arm| arm.body.iter().any(|n| node_matches(n, pred))),
            IrNode::FuncDecl { body, .. } => body.iter().any(|n| node_matches(n, pred)),
            IrNode::MethodDecl { body, .. } => body.iter().any(|n| node_matches(n, pred)),
            IrNode::ImplBlock { methods, .. } => methods.iter().any(|m| node_matches(m, pred)),
            _ => false,
        }
    }

    #[test]
    fn test_analyze_function_def() {
        let code = r#"
def add(a: int, b: int) -> int:
    return a + b
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert_eq!(ir.len(), 1);

        if let IrNode::FuncDecl {
            name,
            params,
            ret,
            body,
            ..
        } = &ir[0]
        {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(*ret, Type::Int);
            assert_eq!(body.len(), 1);
        }
    }

    #[test]
    fn test_analyze_if_stmt() {
        let code = r#"
if x > 0:
    y = 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert_eq!(ir.len(), 1);

        if let IrNode::If { then_block, .. } = &ir[0] {
            assert_eq!(then_block.len(), 1);
        }
    }

    #[test]
    fn test_analyze_for_loop() {
        let code = r#"
for i in range(10):
    x = i
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert_eq!(ir.len(), 1);

        if let IrNode::For { var, body, .. } = &ir[0] {
            assert_eq!(var, "i");
            assert_eq!(body.len(), 1);
        }
    }

    #[test]
    fn test_analyze_class_def() {
        let code = r#"
class Point:
    x: int
    y: int
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(matches!(&ir[0], IrNode::StructDef { .. }));
    }

    #[test]
    fn test_analyze_if_elif_else() {
        let code = r#"
if x > 0:
    y = 1
elif x < 0:
    y = -1
else:
    y = 0
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        // IRではelifはネストしたelse_blockに変換される
        if let IrNode::If { else_block, .. } = &ir[0] {
            assert!(else_block.is_some());
        }
    }

    #[test]
    fn test_analyze_method_call() {
        let code = r#"
arr: list[int] = [1, 2, 3]
arr.append(4)
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- analyze_stmts テスト ---
    // --- SemanticAnalyzer::new テスト ---
    #[test]
    fn test_semantic_analyzer_new() {
        let analyzer = SemanticAnalyzer::new();
        assert!(analyzer.current_return_type.is_none());
    }

    // --- define テスト ---
    #[test]
    fn test_define_variable() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.define("x", Type::Int, false);
        let info = analyzer.scope.lookup("x");
        assert!(info.is_some());
    }

    // --- For loop variants ---
    #[test]
    fn test_analyze_for_range() {
        let code = r#"
def test():
    for i in range(5):
        x = i
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_for_enumerate() {
        let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    for i, v in enumerate(arr):
        x = i + v
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- ClassDef variants ---
    #[test]
    fn test_analyze_class_with_method() {
        let code = r#"
class Counter:
    count: int
    def increment(self):
        self.count += 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- If statement variants ---
    // --- While loop ---
    // --- Return variants ---
    #[test]
    fn test_analyze_return_none() {
        let code = r#"
def foo():
    return
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        if let IrNode::FuncDecl { body, .. } = &ir[0] {
            assert!(matches!(&body[0], IrNode::Return(_)));
        }
    }

    #[test]
    fn test_analyze_return_string() {
        let code = r#"
def foo() -> str:
    return "hello"
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(matches!(&ir[0], IrNode::FuncDecl { .. }));
    }

    #[test]
    fn test_analyze_return_optional_wraps_some() {
        let code = r#"
from typing import Optional

def foo() -> Optional[int]:
    return 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        if let IrNode::FuncDecl { body, .. } = &ir[0] {
            if let IrNode::Return(Some(expr)) = &body[0] {
                match &expr.kind {
                    IrExprKind::Call { func, .. } => {
                        assert!(matches!(&func.kind, IrExprKind::Var(name) if name == "Some"));
                    }
                    _ => panic!("Expected return Some(...)"),
                }
            } else {
                panic!("Expected Return(Some(...))");
            }
        }
    }

    #[test]
    fn test_analyze_return_optional_none_no_some() {
        let code = r#"
from typing import Optional

def foo() -> Optional[int]:
    return None
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        if let IrNode::FuncDecl { body, .. } = &ir[0] {
            if let IrNode::Return(Some(expr)) = &body[0] {
                assert!(matches!(expr.kind, IrExprKind::NoneLit));
            } else {
                panic!("Expected Return(Some(None))");
            }
        }
    }

    #[test]
    fn test_analyze_try_except_creates_tryblock() {
        let code = r#"
try:
    x = 1
except Exception:
    x = 2
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(ir.iter().any(|n| node_matches(n, &|node| matches!(node, IrNode::TryBlock { .. }))));
    }

    #[test]
    fn test_analyze_raise_statement() {
        let code = r#"
raise ValueError("oops")
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(ir.iter().any(|n| node_matches(n, &|node| matches!(node, IrNode::Raise { .. }))));
    }

    #[test]
    fn test_analyze_while_loop() {
        let code = r#"
while x < 10:
    x = x + 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(ir.iter().any(|n| node_matches(n, &|node| matches!(node, IrNode::While { .. }))));
    }

    #[test]
    fn test_analyze_aug_assign() {
        let code = r#"
x = 1
x += 2
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(ir.iter().any(|n| node_matches(n, &|node| matches!(node, IrNode::AugAssign { .. }))));
    }

    #[test]
    fn test_analyze_multi_assign_tuple() {
        let code = r#"
a, b = (1, 2)
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(ir.iter().any(|n| node_matches(n, &|node| matches!(node, IrNode::MultiAssign { .. } | IrNode::MultiVarDecl { .. }))));
    }

    #[test]
    fn test_analyze_return_none_in_unannotated_func() {
        let code = r#"
def foo():
    return None
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        if let IrNode::FuncDecl { body, .. } = &ir[0] {
            assert!(matches!(&body[0], IrNode::Return(_)));
        }
    }

    #[test]
    fn test_analyze_if_else_statement() {
        let code = r#"
if x:
    y = 1
else:
    y = 2
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(ir.iter().any(|n| node_matches(n, &|node| matches!(node, IrNode::If { else_block: Some(_), .. }))));
    }

    #[test]
    fn test_analyze_with_statement_block() {
        let code = r#"
with open("file.txt") as f:
    x = 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(ir.iter().any(|n| node_matches(n, &|node| matches!(node, IrNode::Block { .. }))));
    }

    #[test]
    fn test_analyze_match_statement() {
        let code = r#"
match x:
    case _:
        y = 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        // Match が未対応の構成でも解析が落ちないことを確認する
        assert!(!ir.is_empty());
    }

    // --- multiple targets in for ---
    #[test]
    fn test_analyze_for_multiple_targets() {
        let code = r#"
def test():
    points: list[tuple[int, int]] = [(1, 2), (3, 4)]
    for x, y in points:
        z = x + y
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- method chaining ---
    #[test]
    fn test_analyze_method_chaining() {
        let code = r#"
def test():
    s = "  hello  ".strip().upper()
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- class with multiple methods ---
    #[test]
    fn test_analyze_class_multiple_methods() {
        let code = r#"
class Calculator:
    value: int
    
    def add(self, x: int):
        self.value += x
    
    def sub(self, x: int):
        self.value -= x
    
    def get(self) -> int:
        return self.value
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- complex for loops ---
    #[test]
    fn test_analyze_for_range_start_end() {
        let code = r#"
def test():
    for i in range(5, 10):
        x = i
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_for_range_start_end_step() {
        let code = r#"
def test():
    for i in range(0, 10, 2):
        x = i
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- class inheritance (basic) ---
    #[test]
    fn test_analyze_class_simple() {
        let code = r#"
class Base:
    x: int
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- return tuple ---
    #[test]
    fn test_analyze_return_tuple() {
        let code = r#"
def divmod_custom(a: int, b: int) -> tuple[int, int]:
    return a // b, a % b
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- if with multiple conditions ---
    #[test]
    fn test_analyze_if_compound_condition() {
        let code = r#"
def test():
    x: int = 5
    y: int = 10
    if x > 0 and y > 0:
        z = x + y
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- while with compound condition ---
    #[test]
    fn test_analyze_while_compound() {
        let code = r#"
def test():
    x: int = 0
    y: int = 10
    while x < 10 and y > 0:
        x += 1
        y -= 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- class with field types ---
    #[test]
    fn test_analyze_class_with_typed_fields() {
        let code = r#"
class Person:
    name: str
    age: int
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- method with multiple params ---
    #[test]
    fn test_analyze_method_many_params() {
        let code = r#"
class Calculator:
    def calc(self, a: int, b: int, c: int) -> int:
        return a + b + c
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- complex return expressions ---
    #[test]
    fn test_analyze_return_call_result() {
        let code = r#"
def helper(x: int) -> int:
    return x * 2

def main_func() -> int:
    return helper(10) + helper(20)
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(ir.len() >= 2);
    }

    // --- ternary return ---
    #[test]
    fn test_analyze_return_ternary() {
        let code = r#"
def max_val(a: int, b: int) -> int:
    return a if a > b else b
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_if_elif() {
        let code = r#"
def test():
    x: int = 5
    if x < 0:
        y = -1
    elif x == 0:
        y = 0
    else:
        y = 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_if_elif_chain() {
        let code = r#"
def classify(x: int) -> str:
    if x < 0:
        return "negative"
    elif x == 0:
        return "zero"
    elif x > 0:
        return "positive"
    return "unknown"
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- more for loop patterns ---
    #[test]
    fn test_analyze_for_list_direct() {
        let code = r#"
def test():
    for x in [1, 2, 3]:
        y = x
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_for_str_direct() {
        let code = r#"
def test():
    for c in "hello":
        print(c)
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- more while patterns ---
    #[test]
    fn test_analyze_while_true() {
        let code = r#"
def test():
    while True:
        break
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_while_false() {
        let code = r#"
def test():
    while False:
        x = 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- more class patterns ---
    #[test]
    fn test_analyze_class_one_field() {
        let code = r#"
class Single:
    value: int
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_class_many_fields() {
        let code = r#"
class Data:
    a: int
    b: str
    c: float
    d: bool
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- class with methods ---
    #[test]
    fn test_analyze_class_counter() {
        let code = r#"
class Counter:
    count: int
    
    def increment(self):
        self.count += 1
    
    def decrement(self):
        self.count -= 1
    
    def get(self) -> int:
        return self.count
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_class_stack() {
        let code = r#"
class Stack:
    items: list[int]
    
    def push(self, item: int):
        self.items.append(item)
    
    def pop(self) -> int:
        return self.items.pop()
    
    def is_empty(self) -> bool:
        return len(self.items) == 0
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- more class patterns ---
    #[test]
    fn test_analyze_class_with_init() {
        let code = r#"
class Point:
    x: int
    y: int
    
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_class_method_self() {
        let code = r#"
class Calculator:
    result: int
    
    def reset(self):
        self.result = 0
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- more while patterns ---
    #[test]
    fn test_analyze_while_complex_condition() {
        let code = r#"
def test():
    i: int = 0
    j: int = 10
    while i < 10 and j > 0 and i != j:
        i += 1
        j -= 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_while_counter() {
        let code = r#"
def count_to(n: int) -> int:
    count: int = 0
    while count < n:
        count += 1
    return count
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    // --- more for patterns ---
    #[test]
    fn test_analyze_for_with_continue() {
        let code = r#"
def sum_odd(n: int) -> int:
    total: int = 0
    for i in range(n):
        if i % 2 == 0:
            continue
        total += i
    return total
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }

    #[test]
    fn test_analyze_for_with_break() {
        let code = r#"
def find_first(arr: list[int], target: int) -> int:
    for i in range(len(arr)):
        if arr[i] == target:
            return i
    return -1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert!(!ir.is_empty());
    }
}
