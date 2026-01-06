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
                        target: Box::new(IrExpr::Var("self".to_string())),
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
                            IrExpr::JsonConversion {
                                target: Box::new(ir_value),
                                convert_to: conv.to_string(),
                            }
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
                    IrExpr::Call {
                        func: Box::new(IrExpr::Var("Some".to_string())),
                        args: vec![ir_value],
                    }
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
                    Ok(IrNode::Expr(IrExpr::MethodCall {
                        target: Box::new(ir_target),
                        method: "insert".to_string(),
                        args: vec![ir_index, ir_value],
                    }))
                } else if matches!(current_target_ty, Type::Any) {
                    // For Any type, use __setitem__ method call
                    Ok(IrNode::Expr(IrExpr::PyO3MethodCall {
                        target: Box::new(ir_target),
                        method: "__setitem__".to_string(),
                        args: vec![ir_index, ir_value],
                    }))
                } else {
                    let final_index = match current_target_ty {
                        Type::List(_) | Type::Tuple(_) | Type::String => IrExpr::Cast {
                            target: Box::new(ir_index),
                            ty: "usize".to_string(),
                        },
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
                    return Ok(IrNode::Expr(IrExpr::MethodCall {
                        target: Box::new(IrExpr::Var(target.clone())),
                        method: "push".to_string(),
                        args: vec![ir_value],
                    }));
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
                                IrExpr::MethodCall {
                                    target: Box::new(IrExpr::Slice {
                                        target: Box::new(ir_value.clone()),
                                        start: Some(Box::new(IrExpr::IntLit(start_idx as i64))),
                                        end: None,
                                        step: None,
                                    }),
                                    method: "to_vec".to_string(),
                                    args: vec![],
                                }
                            } else {
                                // values[i..len-end_offset].to_vec()
                                // Need to calculate end index
                                let len_call = IrExpr::MethodCall {
                                    target: Box::new(ir_value.clone()),
                                    method: "len".to_string(),
                                    args: vec![],
                                };
                                let end_expr = IrExpr::BinOp {
                                    left: Box::new(len_call),
                                    op: IrBinOp::Sub,
                                    right: Box::new(IrExpr::IntLit(end_offset as i64)),
                                };
                                IrExpr::MethodCall {
                                    target: Box::new(IrExpr::Slice {
                                        target: Box::new(ir_value.clone()),
                                        start: Some(Box::new(IrExpr::IntLit(start_idx as i64))),
                                        end: Some(Box::new(end_expr)),
                                        step: None,
                                    }),
                                    method: "to_vec".to_string(),
                                    args: vec![],
                                }
                            };

                            nodes.push(IrNode::VarDecl {
                                name: target.clone(),
                                ty,
                                mutable: false,
                                init: Some(Box::new(slice_expr)),
                            });
                        } else if i < *star_idx {
                            // Before starred: head = values[i]
                            self.scope.define(target, elem_ty.clone(), false);

                            let index_expr = IrExpr::Index {
                                target: Box::new(ir_value.clone()),
                                index: Box::new(IrExpr::Cast {
                                    target: Box::new(IrExpr::IntLit(i as i64)),
                                    ty: "usize".to_string(),
                                }),
                            };

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

                            let len_call = IrExpr::MethodCall {
                                target: Box::new(ir_value.clone()),
                                method: "len".to_string(),
                                args: vec![],
                            };
                            let index_expr = IrExpr::Index {
                                target: Box::new(ir_value.clone()),
                                index: Box::new(IrExpr::Cast {
                                    target: Box::new(IrExpr::BinOp {
                                        left: Box::new(len_call),
                                        op: IrBinOp::Sub,
                                        right: Box::new(IrExpr::IntLit(offset_from_end as i64)),
                                    }),
                                    ty: "usize".to_string(),
                                }),
                            };

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
                            let i1_cast = IrExpr::Cast {
                                target: Box::new(ir_i1),
                                ty: "usize".to_string(),
                            };
                            let i2_cast = IrExpr::Cast {
                                target: Box::new(ir_i2),
                                ty: "usize".to_string(),
                            };

                            return Ok(IrNode::Expr(IrExpr::MethodCall {
                                target: Box::new(ir_target),
                                method: "swap".to_string(),
                                args: vec![i1_cast, i2_cast],
                            }));
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
                for p in params {
                    let base_ty = p
                        .type_hint
                        .as_ref()
                        .map(|th| self.type_from_hint(th))
                        .unwrap_or(Type::Unknown);

                    // For variadic parameters (*args), wrap in Vec<T>
                    let ty = if p.variadic {
                        Type::List(Box::new(base_ty))
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

                // Define function in current scope BEFORE analyzing body (for recursion)
                self.scope.define(
                    name,
                    Type::Func {
                        params: param_types.clone(),
                        ret: Box::new(resolved_ret_type.clone()),
                        is_boxed: false,
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
                    self.scope.pop();

                    // Warn about closures if capturing variables?
                    // Currently implicit capture via 'move' in Rust.

                    let closure = IrExpr::Closure {
                        params: param_names,
                        body: ir_body,
                        ret_type: ret_type.clone(),
                    };

                    // Wrap closure in Box::new(...) to match Type::Func (Box<dyn Fn...>)
                    let boxed_closure = IrExpr::BoxNew(Box::new(closure));

                    return Ok(IrNode::VarDecl {
                        name: name.clone(),
                        ty: Type::Func {
                            params: param_types,
                            ret: Box::new(resolved_ret_type),
                            is_boxed: true,
                        }, // Variable holding closure is Boxed
                        mutable: false,
                        init: Some(Box::new(boxed_closure)),
                    });
                }

                self.scope.push();
                
                // Clear hoisted_var_candidates for this function scope
                self.hoisted_var_candidates.clear();
                self.func_base_depth = self.scope.depth();

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

                self.scope.pop();

                // Collect hoisted variables: those used at shallower depth than defined
                let hoisted_vars: Vec<HoistedVar> = self.hoisted_var_candidates
                    .drain()
                    .map(|(name, (ty, _, _))| HoistedVar { name, ty })
                    .collect();

                let ir_name = name.clone();
                Ok(IrNode::FuncDecl {
                    name: ir_name,
                    params: ir_params,
                    ret: ret_type,
                    body: ir_body,
                    hoisted_vars,
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
                                hoisted_vars: vec![],  // main 関数用
                            });
                        }
                    }
                }

                // Check for type narrowing pattern: `if x is None:` or `if x is not None:`
                // Extract variable name and narrowing direction
                let narrowing_info = self.extract_none_check(condition);

                let ir_cond = self.analyze_expr(condition)?;

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

                        // Check if we're returning from an Optional function
                        let is_optional_return =
                            matches!(&self.current_return_type, Some(Type::Optional(_)));

                        // If returning a Reference to a List (slice), use .to_vec() to return owned
                        let ir = if let Type::Ref(inner) = &ty {
                            if matches!(inner.as_ref(), Type::List(_)) {
                                IrExpr::MethodCall {
                                    target: Box::new(ir),
                                    method: "to_vec".to_string(),
                                    args: vec![],
                                }
                            } else {
                                IrExpr::MethodCall {
                                    target: Box::new(ir),
                                    method: "clone".to_string(),
                                    args: vec![],
                                }
                            }
                        } else {
                            ir
                        };

                        // If returning a string literal to a String return type, add .to_string()
                        let ir = if matches!(self.current_return_type, Some(Type::String))
                            && matches!(ir, IrExpr::StringLit(_))
                        {
                            IrExpr::MethodCall {
                                target: Box::new(ir),
                                method: "to_string".to_string(),
                                args: vec![],
                            }
                        } else {
                            ir
                        };

                        // Wrap in Some() if returning to Optional and value is not None
                        if is_optional_return && !matches!(ir, IrExpr::NoneLit) {
                            Some(Box::new(IrExpr::Call {
                                func: Box::new(IrExpr::Var("Some".to_string())),
                                args: vec![ir],
                            }))
                        } else if matches!(ty, Type::Any) {
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
                                    Some(Box::new(IrExpr::JsonConversion {
                                        target: Box::new(ir),
                                        convert_to: conv.to_string(),
                                    }))
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
                fields,
                methods,
            } => {
                // Convert AST fields to IR fields with types
                let ir_fields: Vec<(String, Type)> = fields
                    .iter()
                    .map(|f| {
                        let ty = self.type_from_hint(&f.type_hint);
                        (f.name.clone(), ty)
                    })
                    .collect();

                // Save field types for constructor type checking
                self.struct_field_types
                    .insert(name.clone(), ir_fields.clone());

                // Register this struct type in scope (for use in type hints)
                self.scope.define(name, Type::Struct(name.clone()), false);

                // If there are methods, create an impl block
                let mut result_nodes = vec![IrNode::StructDef {
                    name: name.clone(),
                    fields: ir_fields.clone(),
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
                        // Define method params
                        for (p_name, p_ty) in &ir_params {
                            self.scope.define(p_name, p_ty.clone(), false);
                        }

                        let ir_body: Vec<IrNode> = method
                            .body
                            .iter()
                            .map(|s| self.analyze_stmt(s))
                            .collect::<Result<Vec<_>, _>>()?;
                        self.scope.pop();

                        // Check if method modifies self (contains FieldAssign)
                        let takes_mut_self = ir_body
                            .iter()
                            .any(|node| matches!(node, IrNode::FieldAssign { .. }));

                        ir_methods.push(IrNode::MethodDecl {
                            name: method.name.clone(),
                            params: ir_params,
                            ret: ret_ty,
                            body: ir_body,
                            takes_self: !method.is_static,
                            takes_mut_self,
                        });
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
            } => {
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
                })
            }
            Stmt::TryExcept {
                try_body,
                except_clauses,
                else_body,      // V1.5.2: else ブロック
                finally_body,
            } => {
                // Use analyze_stmts to properly detect mutable variables in try/except blocks
                let ir_try_body = self.analyze_stmts(try_body)?;

                // V1.5.2: Collect the first except clause's variable name for IR
                let except_var = except_clauses
                    .iter()
                    .find_map(|c| c.name.clone());

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
                    except_var,       // V1.5.2: 例外変数名
                    else_body: ir_else_body,  // V1.5.2: else ブロック
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
                // V1.4.0: Register external imports for non-native modules
                // Native modules (math, etc.) are handled directly in Rust,
                // so they should NOT be registered as external imports.

                // Native module whitelist - these are converted to Rust native code
                const NATIVE_MODULES: &[&str] = &["math", "typing"];

                if !NATIVE_MODULES.contains(&module.as_str()) {
                    // Non-native modules go through Resident Worker
                    if let Some(ref item_list) = items {
                        // "from module import a, b, c" - register each item as (module, item)
                        for item in item_list {
                            self.external_imports.push((module.clone(), item.clone()));
                        }
                    } else {
                        // "import module" or "import module as alias"
                        let effective_name = alias.as_ref().unwrap_or(module);
                        self.external_imports
                            .push((module.clone(), effective_name.clone()));
                    }
                }

                // For now, return an empty sequence (no IR generated)
                // The PyO3 wrapper will be added in emit phase
                Ok(IrNode::PyO3Import {
                    module: module.clone(),
                    alias: alias.clone(),
                    items: items.clone(),
                })
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::semantic::analyze;

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
            hoisted_vars: _,
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
