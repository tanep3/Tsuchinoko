//! Call and method handling for SemanticAnalyzer
//!
//! Extracted from mod.rs for maintainability

use super::*;

impl SemanticAnalyzer {
    pub(crate) fn analyze_call_args(
        &mut self,
        args: &[Expr],
        expected_param_types: &[Type],
        _func_name: &str,
    ) -> Result<Vec<IrExpr>, TsuchinokoError> {
        let mut ir_args = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let ir_arg = self.analyze_expr(a)?;
            let actual_ty = self.infer_type(a);
            let expected_ty = expected_param_types
                .get(i)
                .cloned()
                .unwrap_or(Type::Unknown);

            let coerced = self.coerce_arg(ir_arg, &actual_ty, &expected_ty, a);
            ir_args.push(coerced);
        }
        Ok(ir_args)
    }

    /// Coerce a single argument to match the expected type
    /// Handles Auto-Box, Auto-Ref, Auto-Deref, and Fallback Clone
    pub(crate) fn coerce_arg(
        &mut self,
        mut ir_arg: IrExpr,
        actual_ty: &Type,
        expected_ty: &Type,
        expr: &Expr,
    ) -> IrExpr {
        // 1. Unpack expectation (check if expected type is a reference or mutable reference)
        let (target_ty, needs_ref, needs_mut_ref) = match expected_ty {
            Type::MutRef(inner) => (inner.as_ref().clone(), false, true),
            Type::Ref(inner) => (inner.as_ref().clone(), true, false),
            _ => (expected_ty.clone(), false, false),
        };

        let resolved_target = self.resolve_type(&target_ty);
        let mut resolved_actual = self.resolve_type(actual_ty);

        // Strip all references from actual for comparison
        while let Type::Ref(inner) = resolved_actual {
            resolved_actual = *inner;
        }

        // 1.5 Auto-Some: T -> Option<T>
        // If expected is Option<T> and actual is T (not None), wrap in Some()
        if let Type::Optional(inner_expected) = &resolved_target {
            // Check if actual is NOT None/Optional
            if !matches!(resolved_actual, Type::Optional(_)) && !matches!(expr, Expr::NoneLiteral) {
                // If inner is String and arg is a string literal, add .to_string()
                let wrapped_arg = if matches!(inner_expected.as_ref(), Type::String)
                    && matches!(ir_arg.kind, IrExprKind::StringLit(_))
                {
                    self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_arg),
                        method: "to_string".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::String)
                } else {
                    ir_arg
                };

                // Wrap the argument in Some()
                let some_func = self.create_expr(IrExprKind::Var("Some".to_string()), Type::Unknown);
                return self.create_expr(IrExprKind::Call {
                    func: Box::new(some_func),
                    args: vec![wrapped_arg],
                    callee_may_raise: false,
                    callee_needs_bridge: false,
                }, resolved_target);
            }
            // If actual is also Optional or None, use as-is
            _ = inner_expected; // Suppress unused warning
        }

        // 2. Auto-Box: Fn -> Box<dyn Fn>
        if let Type::Func { is_boxed: true, .. } = &resolved_target {
            if let Type::Func {
                is_boxed: false, ..
            } = &resolved_actual
            {
                ir_arg = self.create_expr(IrExprKind::BoxNew(Box::new(ir_arg)), resolved_target.clone());

                // If target was a named alias, add explicit cast
                if let Type::Struct(alias_name) = &target_ty {
                    ir_arg = self.create_expr(IrExprKind::Cast {
                        target: Box::new(ir_arg),
                        ty: alias_name.clone(),
                    }, target_ty.clone());
                }
                return ir_arg;
            }
        }

        // 3. Auto-Ref for Index expressions
        if needs_ref && matches!(expr, Expr::Index { .. }) {
            return self.create_expr(IrExprKind::Reference {
                target: Box::new(ir_arg),
            }, expected_ty.clone());
        }

        // 3.5 Auto-MutRef for mutable reference parameters
        if needs_mut_ref {
            // Need a mutable reference
            return self.create_expr(IrExprKind::MutReference {
                target: Box::new(ir_arg),
            }, expected_ty.clone());
        }

        // 4. Auto-Ref/Deref logic
        if needs_ref {
            // Need a reference
            if let Type::Ref(_) = actual_ty {
                // Already a reference, use as-is
                ir_arg
            } else {
                // Not a reference, add one
                self.create_expr(IrExprKind::Reference {
                    target: Box::new(ir_arg),
                }, expected_ty.clone())
            }
        } else {
            // Need an owned value - apply Auto-Deref for Copy types
            let mut current_ty = actual_ty.clone();
            while let Type::Ref(inner) = &current_ty {
                let inner_ty = inner.as_ref();
                if inner_ty.is_copy() {
                    ir_arg = self.create_expr(IrExprKind::UnaryOp {
                        op: IrUnaryOp::Deref,
                        operand: Box::new(ir_arg),
                    }, inner_ty.clone());
                    current_ty = inner_ty.clone();
                    // If expected type is Unknown or compatible, we're done
                    if resolved_target == Type::Unknown
                        || current_ty.is_compatible_with(&resolved_target)
                    {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Fallback Clone for non-Copy types
            // Skip clone for method calls that return Copy types (like len())
            let is_copy_method =
                matches!(&ir_arg.kind, IrExprKind::MethodCall { method, .. } if method == "len");
            if !resolved_actual.is_copy()
                && !matches!(actual_ty, Type::Ref(_))
                && !matches!(resolved_actual, Type::Func { .. })
                && !is_copy_method
            {
                let method = if matches!(ir_arg.kind, IrExprKind::StringLit(_) | IrExprKind::FString { .. }) {
                    "to_string"
                } else {
                    "clone"
                };
                ir_arg = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(ir_arg),
                    method: method.to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, resolved_actual.clone());
            }

            // Special case: &String -> String
            if let Type::Ref(inner) = actual_ty {
                if **inner == Type::String {
                    ir_arg = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_arg),
                        method: "to_string".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::String);
                }
            }

            ir_arg
        }
    }

    /// Analyze for loop iterator, handling enumerate() and zip() (V1.3.0)
    /// Returns: (actual_target, ir_iter, elem_type)
    pub(crate) fn analyze_for_iter(
        &mut self,
        target: &str,
        iter: &Expr,
    ) -> Result<(String, IrExpr, Type), TsuchinokoError> {
        // Check for enumerate(iterable) or enumerate(iterable, start=N)
        if let Expr::Call { func, args, kwargs } = iter {
            if let Expr::Ident(func_name) = func.as_ref() {
                // enumerate(iterable) or enumerate(iterable, start=N) -> iterable.iter().enumerate()
                if func_name == "enumerate" && !args.is_empty() {
                    let iterable = &args[0];
                    let iterable_ty = self.infer_type(iterable);
                    let ir_iterable = self.analyze_expr(iterable)?;

                    // Check for start argument (as positional arg[1] or kwargs["start"])
                    let start_value: Option<i64> = if args.len() > 1 {
                        // enumerate(iterable, 1) - positional start
                        if let Expr::IntLiteral(n) = &args[1] {
                            Some(*n)
                        } else {
                            None
                        }
                    } else if let Some((_, Expr::IntLiteral(n))) =
                        kwargs.iter().find(|(k, _)| k == "start")
                    {
                        // enumerate(iterable, start=1) - keyword start
                        Some(*n)
                    } else {
                        None
                    };

                    // Get the element type from the iterable
                    let inner_elem_type = match &iterable_ty {
                        Type::List(elem) => elem.as_ref().clone(),
                        Type::Ref(inner) => {
                            if let Type::List(elem) = inner.as_ref() {
                                elem.as_ref().clone()
                            } else {
                                Type::Unknown
                            }
                        }
                        _ => Type::Unknown,
                    };

                    // Build: iterable.iter().enumerate().map(|(i, x)| (i as i64 + start, x.clone()))
                    let iter_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_iterable),
                        method: "iter".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    let enumerate_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(iter_call),
                        method: "enumerate".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);

                    // Add .map() to convert (usize, &T) -> (i64, T) with optional start offset
                    let map_closure = if let Some(start) = start_value {
                        format!("|(i, x)| (i as i64 + {start}, x.clone())")
                    } else {
                        "|(i, x)| (i as i64, x.clone())".to_string()
                    };
                    let map_fn = self.create_expr(IrExprKind::RawCode(map_closure), Type::Unknown);
                    let mapped_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(enumerate_call),
                        method: "map".to_string(),
                        args: vec![map_fn],
                        callee_needs_bridge: false,
                    }, Type::Unknown);

                    // Element type is (i64, T)
                    let elem_type = Type::Tuple(vec![Type::Int, inner_elem_type]);

                    return Ok((target.to_string(), mapped_call, elem_type));
                }

                // V1.3.0: reversed(iterable) -> iterable.iter().rev().cloned()
                // For strings: s.chars().rev()
                if func_name == "reversed" && !args.is_empty() && kwargs.is_empty() {
                    let iterable = &args[0];
                    let mut iterable_ty = self.infer_type(iterable);
                    // Unwrap Ref
                    while let Type::Ref(inner) = iterable_ty {
                        iterable_ty = *inner;
                    }
                    let ir_iterable = self.analyze_expr(iterable)?;

                    // Handle string separately - use .chars().rev()
                    if matches!(iterable_ty, Type::String) {
                        let chars_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(ir_iterable),
                            method: "chars".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        let rev_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(chars_call),
                            method: "rev".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        return Ok((target.to_string(), rev_call, Type::Unknown));
                    }

                    let inner_elem_type = match &iterable_ty {
                        Type::List(elem) => elem.as_ref().clone(),
                        _ => Type::Unknown,
                    };

                    // Build: iterable.iter().rev().cloned()
                    let iter_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_iterable),
                        method: "iter".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    let rev_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(iter_call),
                        method: "rev".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    let cloned_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(rev_call),
                        method: "cloned".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, inner_elem_type.clone());

                    return Ok((target.to_string(), cloned_call, inner_elem_type));
                }

                // zip(a, b) -> a.iter().zip(b.iter()).map(|(x, y)| (x.clone(), y.clone()))
                if func_name == "zip" && args.len() >= 2 && kwargs.is_empty() {
                    let first_iterable = &args[0];
                    let ir_first = self.analyze_expr(first_iterable)?;
                    let first_ty = self.infer_type(first_iterable);

                    // Start with first.iter()
                    let mut ir_iter = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_first),
                        method: "iter".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);

                    // Collect element types for tuple
                    let mut elem_types = vec![self.get_elem_type(&first_ty)];
                    let num_args = args.len();

                    // Chain .zip() for each additional iterable
                    for arg in args.iter().skip(1) {
                        let ir_arg = self.analyze_expr(arg)?;
                        let arg_ty = self.infer_type(arg);
                        elem_types.push(self.get_elem_type(&arg_ty));

                        // .zip(arg.iter())
                        let arg_iter = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(ir_arg),
                            method: "iter".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        ir_iter = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(ir_iter),
                            method: "zip".to_string(),
                            args: vec![arg_iter],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                    }

                    // Add .map() to clone references
                    // For 2 args: .map(|(x, y)| (x.clone(), y.clone()))
                    // For 3 args: .map(|((x, y), z)| (x.clone(), y.clone(), z.clone()))
                    let map_closure = if num_args == 2 {
                        "|(x, y)| (x.clone(), y.clone())".to_string()
                    } else if num_args == 3 {
                        "|((x, y), z)| (x.clone(), y.clone(), z.clone())".to_string()
                    } else {
                        // Fallback for more args - just pass through
                        "|(x, y)| (x.clone(), y.clone())".to_string()
                    };

                    let map_fn = self.create_expr(IrExprKind::RawCode(map_closure), Type::Unknown);
                    ir_iter = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(ir_iter),
                        method: "map".to_string(),
                        args: vec![map_fn],
                        callee_needs_bridge: false,
                    }, Type::Unknown);

                    let elem_type = Type::Tuple(elem_types);
                    return Ok((target.to_string(), ir_iter, elem_type));
                }
            }
        }

        // Default case: standard iteration
        let mut ir_iter = self.analyze_expr(iter)?;
        let mut iter_type = self.infer_type(iter);

        // If iterating over a Reference to a List, iterate over cloned elements
        if let Type::Ref(inner) = &iter_type {
            if let Type::List(_) = **inner {
                ir_iter = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(ir_iter),
                    method: "iter".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, Type::Unknown);
                ir_iter = self.create_expr(IrExprKind::MethodCall {
                    target_type: Type::Unknown,
                    target: Box::new(ir_iter),
                    method: "cloned".to_string(),
                    args: vec![],
                    callee_needs_bridge: false,
                }, inner.as_ref().clone());
                iter_type = *inner.clone();
            }
        }

        let elem_type = if let Type::List(elem) = iter_type {
            *elem
        } else {
            Type::Int // Default fallback for range()
        };

        Ok((target.to_string(), ir_iter, elem_type))
    }

    /// Helper to get element type from a list/ref type
    pub(crate) fn get_elem_type(&self, ty: &Type) -> Type {
        match ty {
            Type::List(elem) => elem.as_ref().clone(),
            Type::Ref(inner) => {
                if let Type::List(elem) = inner.as_ref() {
                    elem.as_ref().clone()
                } else {
                    Type::Unknown
                }
            }
            _ => Type::Unknown,
        }
    }

    /// Define loop variables in scope based on iterator type
    /// Used by ListComp, GenExpr, and For loops
    pub(crate) fn define_loop_variables(
        &mut self,
        target: &str,
        iter_ty: &Type,
        wrap_in_ref: bool,
    ) {
        match iter_ty {
            Type::List(inner) => {
                let elem_ty = inner.as_ref().clone();
                self.define_loop_vars_from_elem(target, &elem_ty, wrap_in_ref);
            }
            Type::Dict(k, v) => {
                // For .items() iteration
                if target.contains(',') {
                    let targets: Vec<_> = target.split(',').map(|s| s.trim()).collect();
                    if targets.len() == 2 {
                        self.scope.define(targets[0], Type::Ref(k.clone()), false);
                        self.scope.define(targets[1], Type::Ref(v.clone()), false);
                    }
                }
            }
            Type::Tuple(elems) => {
                if target.contains(',') {
                    let targets: Vec<_> = target.split(',').map(|s| s.trim()).collect();
                    for (t, ty) in targets.iter().zip(elems.iter()) {
                        let final_ty = if wrap_in_ref && !matches!(ty, Type::Ref(_)) {
                            Type::Ref(Box::new(ty.clone()))
                        } else {
                            ty.clone()
                        };
                        self.scope.define(t, final_ty, false);
                    }
                }
            }
            _ => {
                // Unknown or Range type
                if target.contains(',') {
                    for t in target.split(',') {
                        self.scope.define(t.trim(), Type::Unknown, false);
                    }
                } else {
                    let ty = if wrap_in_ref {
                        Type::Ref(Box::new(Type::Int))
                    } else {
                        Type::Int
                    };
                    self.scope.define(target, ty, false);
                }
            }
        }
    }

    /// Helper to define loop variables from element type (handles tuple unpacking)
    pub(crate) fn define_loop_vars_from_elem(
        &mut self,
        target: &str,
        elem_ty: &Type,
        wrap_in_ref: bool,
    ) {
        if target.contains(',') {
            // Tuple unpacking: (k, v) for t in list_of_tuples
            if let Type::Tuple(elems) = elem_ty {
                let targets: Vec<_> = target.split(',').map(|s| s.trim()).collect();
                for (t, ty) in targets.iter().zip(elems.iter()) {
                    let final_ty = if wrap_in_ref && !matches!(ty, Type::Ref(_)) {
                        Type::Ref(Box::new(ty.clone()))
                    } else {
                        ty.clone()
                    };
                    self.scope.define(t, final_ty, false);
                }
            } else {
                // Fallback for non-tuple
                for t in target.split(',') {
                    self.scope.define(t.trim(), Type::Unknown, false);
                }
            }
        } else {
            // Single variable
            let final_ty = if wrap_in_ref && !matches!(elem_ty, Type::Ref(_)) {
                Type::Ref(Box::new(elem_ty.clone()))
            } else {
                elem_ty.clone()
            };
            self.scope.define(target, final_ty, false);
        }
    }

    /// Handle special method calls that require transformation
    /// Returns Some(IrExpr) if the method was handled specially, None otherwise
    pub(crate) fn try_handle_special_method(
        &mut self,
        target_ir: &IrExpr,
        _target_ty: &Type,
        method: &str,
        args: &[Expr],
    ) -> Result<Option<IrExpr>, TsuchinokoError> {
        match method {
            "items" if args.is_empty() => {
                // dict.items() -> dict.iter().map(|(k,v)|(*k, v.clone())) (only for Dict types)
                // For user-defined structs with items() method, keep the call as-is
                // Explicitly handle both Dict and non-Dict cases
                match _target_ty {
                    Type::Dict(_, _) => {
                        // dict.iter() returns (&K, &V), we need owned (K, V) for filter/map
                        // Use iter().map() to clone values for ownership
                        let iter_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(target_ir.clone()),
                            method: "iter".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        let map_fn = self.create_expr(IrExprKind::RawCode("|(k, v)| (*k, v.clone())".to_string()), Type::Unknown);
                        Ok(Some(self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(iter_call),
                            method: "map".to_string(),
                            args: vec![map_fn],
                            callee_needs_bridge: false,
                        }, Type::Unknown)))
                    }
                    Type::Struct(_) => {
                        // User-defined struct with items() method - keep as-is
                        Ok(None)
                    }
                    _ => {
                        // Unknown or other type - don't transform
                        Ok(None)
                    }
                }
            }
            // V1.5.0: dict.keys() -> dict.keys().cloned()
            "keys" if args.is_empty() => match _target_ty {
                Type::Dict(_, _) => {
                    let keys_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(target_ir.clone()),
                        method: "keys".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    Ok(Some(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(keys_call),
                        method: "cloned".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown)))
                }
                _ => Ok(None),
            },
            // V1.5.0: dict.values() -> dict.values().cloned()
            "values" if args.is_empty() => match _target_ty {
                Type::Dict(_, _) => {
                    let values_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(target_ir.clone()),
                        method: "values".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    Ok(Some(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(values_call),
                        method: "cloned".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown)))
                }
                _ => Ok(None),
            },
            // V1.5.0: dict.get(k) -> dict.get(&k).cloned().unwrap()
            // V1.5.0: dict.get(k, default) -> dict.get(&k).cloned().unwrap_or(default)
            // V1.6.0: kwargs (Dict<String, Any>) の場合は and_then(|v| v.as_T()).unwrap_or(default)
            "get" if !args.is_empty() => {
                // Unwrap Ref if present
                let inner_ty = match _target_ty {
                    Type::Ref(inner) => inner.as_ref(),
                    _ => _target_ty,
                };

                match inner_ty {
                    // V1.6.0 FT-006: kwargs (Dict<String, Any>) の場合は特殊処理
                    Type::Dict(_, value_ty) if matches!(value_ty.as_ref(), Type::Any) => {
                        let key = self.analyze_expr(&args[0])?;
                        let get_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(target_ir.clone()),
                            method: "get".to_string(),
                            // kwargs (HashMap<String, Value>) の場合、get は &str を受け付ける
                            args: vec![key],
                            callee_needs_bridge: false,
                        }, Type::Unknown);

                        if args.len() >= 2 {
                            // get(k, default) -> get(&k).and_then(|v| v.as_T()).unwrap_or(default)
                            let default = self.analyze_expr(&args[1])?;
                            let default_ty = self.infer_type(&args[1]);

                            // Determine the appropriate as_* method based on default type
                            let (as_method, default_ir, result_ty) = match &default_ty {
                                Type::String => ("as_str", default, Type::String),
                                Type::Bool => ("as_bool", default, Type::Bool),
                                Type::Int => ("as_i64", default, Type::Int),
                                Type::Float => ("as_f64", default, Type::Float),
                                _ => ("as_str", default, Type::String), // default to string
                            };

                            // Build: kwargs.get(&key).and_then(|v| v.as_T()).unwrap_or(default)
                            // For strings: kwargs.get(&key).and_then(|v| v.as_str()).unwrap_or("default").to_string()
                            let map_fn = self.create_expr(IrExprKind::RawCode(format!(
                                "|v: &TnkValue| v.{}()",
                                as_method
                            )), Type::Unknown);
                            let and_then_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(get_call),
                                method: "and_then".to_string(),
                                args: vec![map_fn],
                                callee_needs_bridge: false,
                            }, Type::Unknown);

                            let unwrap_call = if matches!(result_ty, Type::String) {
                                // For string: unwrap_or("default").to_string()
                                let unwrap = self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(and_then_call),
                                    method: "unwrap_or".to_string(),
                                    args: vec![default_ir],
                                    callee_needs_bridge: false,
                                }, Type::Unknown);
                                self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(unwrap),
                                    method: "to_string".to_string(),
                                    args: vec![],
                                    callee_needs_bridge: false,
                                }, Type::String)
                            } else {
                                self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(and_then_call),
                                    method: "unwrap_or".to_string(),
                                    args: vec![default_ir],
                                    callee_needs_bridge: false,
                                }, result_ty.clone())
                            };

                            Ok(Some(unwrap_call))
                        } else {
                            // get(k) without default - this is less common for kwargs
                            let cloned_call = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(get_call),
                                method: "cloned".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown);
                            Ok(Some(self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(cloned_call),
                                method: "unwrap".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown)))
                        }
                    }
                    Type::Dict(_, _) => {
                        let key = self.analyze_expr(&args[0])?;
                        let key_ref = self.create_expr(IrExprKind::Reference {
                            target: Box::new(key),
                        }, Type::Unknown);
                        let get_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(target_ir.clone()),
                            method: "get".to_string(),
                            args: vec![key_ref],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        let cloned_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(get_call),
                            method: "cloned".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        if args.len() >= 2 {
                            // get(k, default) -> get(&k).cloned().unwrap_or(default)
                            let mut default = self.analyze_expr(&args[1])?;
                            // Convert string literals to String for type compatibility
                            if matches!(default.kind, IrExprKind::StringLit(_)) {
                                default = self.create_expr(IrExprKind::MethodCall {
                                    target_type: Type::Unknown,
                                    target: Box::new(default),
                                    method: "to_string".to_string(),
                                    args: vec![],
                                    callee_needs_bridge: false,
                                }, Type::String);
                            }
                            Ok(Some(self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(cloned_call),
                                method: "unwrap_or".to_string(),
                                args: vec![default],
                                callee_needs_bridge: false,
                            }, Type::Unknown)))
                        } else {
                            // get(k) -> get(&k).cloned().unwrap()
                            Ok(Some(self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Unknown,
                                target: Box::new(cloned_call),
                                method: "unwrap".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Unknown)))
                        }
                    }
                    _ => Ok(None),
                }
            }
            // V1.5.0: dict.pop(k) -> dict.remove(&k).unwrap()
            "pop" if args.len() == 1 => {
                match _target_ty {
                    Type::Dict(_, _) => {
                        let key = self.analyze_expr(&args[0])?;
                        let key_ref = self.create_expr(IrExprKind::Reference {
                            target: Box::new(key),
                        }, Type::Unknown);
                        let remove_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(target_ir.clone()),
                            method: "remove".to_string(),
                            args: vec![key_ref],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        Ok(Some(self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(remove_call),
                            method: "unwrap".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown)))
                    }
                    _ => Ok(None), // list.pop is handled by emitter
                }
            }
            // V1.5.0: dict.update(other) -> dict.extend(other)
            "update" if args.len() == 1 => match _target_ty {
                Type::Dict(_, _) => {
                    let other = self.analyze_expr(&args[0])?;
                    Ok(Some(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(target_ir.clone()),
                        method: "extend".to_string(),
                        args: vec![other],
                        callee_needs_bridge: false,
                    }, Type::Unit)))
                }
                _ => Ok(None),
            },
            "join" if args.len() == 1 => {
                // "sep".join(iterable) -> iterable.iter().map(|x| x.to_string()).collect().join(&sep)
                let iterable_ast = &args[0];
                let iterable_ty = self.infer_type(iterable_ast);
                let iterable_ir = self.analyze_expr(iterable_ast)?;

                let needs_string_conversion = match &iterable_ty {
                    Type::List(inner) => **inner != Type::String,
                    _ => true,
                };

                if needs_string_conversion {
                    let iter_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(iterable_ir),
                        method: "iter".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    let x_var = self.create_expr(IrExprKind::Var("x".to_string()), Type::Unknown);
                    let to_string_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(x_var),
                        method: "to_string".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::String);
                    let closure = self.create_expr(IrExprKind::Closure {
                        params: vec!["x".to_string()],
                        body: vec![IrNode::Expr(to_string_call)],
                        ret_type: Type::String,
                    }, Type::Func {
                        params: vec![Type::Unknown],
                        ret: Box::new(Type::String),
                        may_raise: false,
                        is_boxed: true,
                    });
                    let map_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(iter_call),
                        method: "map".to_string(),
                        args: vec![closure],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    let collect_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(map_call),
                        method: "collect::<Vec<String>>".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::List(Box::new(Type::String)));
                    Ok(Some(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(collect_call),
                        method: "join".to_string(),
                        args: vec![target_ir.clone()],
                        callee_needs_bridge: false,
                    }, Type::String)))
                } else {
                    Ok(Some(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(iterable_ir),
                        method: "join".to_string(),
                        args: vec![target_ir.clone()],
                        callee_needs_bridge: false,
                    }, Type::String)))
                }
            }
            // V1.3.0: list.index(x) -> list.iter().position(|e| *e == x).unwrap() as i64
            "index" if args.len() == 1 => {
                match _target_ty {
                    Type::List(_) | Type::Ref(_) => {
                        let search_val = self.analyze_expr(&args[0])?;
                        // list.iter().position(|e| *e == val).unwrap() as i64
                        let iter_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(target_ir.clone()),
                            method: "iter".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        let filter_fn = self.create_expr(IrExprKind::RawCode(format!(
                            "|e| *e == {}",
                            self.emit_simple_expr(&search_val)
                        )), Type::Unknown);
                        let position_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(iter_call),
                            method: "position".to_string(),
                            args: vec![filter_fn],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        let unwrap_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(position_call),
                            method: "unwrap".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        // Cast to i64 (wrap in parens for correct precedence)
                        Ok(Some(self.create_expr(IrExprKind::RawCode(format!(
                            "({} as i64)",
                            self.emit_simple_ir_expr(&unwrap_call)
                        )), Type::Int)))
                    }
                    _ => Ok(None),
                }
            }
            // V1.3.0: list.count(x) -> list.iter().filter(|e| **e == x).count() as i64
            "count" if args.len() == 1 => {
                match _target_ty {
                    Type::List(_) | Type::Ref(_) => {
                        let search_val = self.analyze_expr(&args[0])?;
                        // list.iter().filter(|e| **e == val).count() as i64
                        let iter_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(target_ir.clone()),
                            method: "iter".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        let filter_fn = self.create_expr(IrExprKind::RawCode(format!(
                            "|e| **e == {}",
                            self.emit_simple_expr(&search_val)
                        )), Type::Unknown);
                        let filter_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(iter_call),
                            method: "filter".to_string(),
                            args: vec![filter_fn],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        let count_call = self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(filter_call),
                            method: "count".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::Unknown);
                        // Cast to i64 (wrap in parens for correct precedence)
                        Ok(Some(self.create_expr(IrExprKind::RawCode(format!(
                            "({} as i64)",
                            self.emit_simple_ir_expr(&count_call)
                        )), Type::Int)))
                    }
                    _ => Ok(None),
                }
            }
            // V1.5.0: list.remove(x) -> remove by value (find position first)
            // list.remove(x) -> { let pos = list.iter().position(|e| *e == x).unwrap(); list.remove(pos); }
            // For now, generate: list.retain(|e| *e != x) (removes ALL occurrences - different semantics)
            // Better: use RawCode for inline block
            "remove" if args.len() == 1 => {
                // Check if this is a list type (including Ref<List>)
                let is_list = matches!(_target_ty, Type::List(_))
                    || matches!(_target_ty, Type::Ref(inner) if matches!(inner.as_ref(), Type::List(_)));

                if is_list {
                    let search_val = self.analyze_expr(&args[0])?;
                    // Python list.remove(x) removes FIRST occurrence only
                    // Rust: let pos = list.iter().position(|e| *e == val).unwrap(); list.remove(pos);
                    // Since this is a statement expression, we generate:
                    // list.remove(list.iter().position(|e| *e == val).unwrap())
                    let iter_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(target_ir.clone()),
                        method: "iter".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    let filter_fn = self.create_expr(IrExprKind::RawCode(format!(
                        "|e| *e == {}",
                        self.emit_simple_expr(&search_val)
                    )), Type::Unknown);
                    let position_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(iter_call),
                        method: "position".to_string(),
                        args: vec![filter_fn],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    let unwrap_call = self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(position_call),
                        method: "unwrap".to_string(),
                        args: vec![],
                        callee_needs_bridge: false,
                    }, Type::Unknown);
                    Ok(Some(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(target_ir.clone()),
                        method: "remove".to_string(),
                        args: vec![unwrap_call],
                        callee_needs_bridge: false,
                    }, Type::Unit)))
                } else {
                    // Check if this is a set type
                    let is_set = matches!(_target_ty, Type::Set(_))
                        || matches!(_target_ty, Type::Ref(inner) if matches!(inner.as_ref(), Type::Set(_)));

                    if is_set {
                        // Python set.remove(x) -> Rust set.remove(&x)
                        let search_val = self.analyze_expr(&args[0])?;
                        let ref_val = self.create_expr(IrExprKind::Reference {
                            target: Box::new(search_val),
                        }, Type::Unknown);
                        Ok(Some(self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(target_ir.clone()),
                            method: "remove".to_string(),
                            args: vec![ref_val],
                            callee_needs_bridge: false,
                        }, Type::Bool)))
                    } else {
                        Ok(None) // Not a list or set, fall through to default handling
                    }
                }
            }
            // V1.5.0: list.insert(i, x) -> list.insert(i as usize, x)
            // Only apply to list types, not dict
            "insert" if args.len() == 2 => {
                let is_list = matches!(_target_ty, Type::List(_))
                    || matches!(_target_ty, Type::Ref(inner) if matches!(inner.as_ref(), Type::List(_)));

                if is_list {
                    let idx = self.analyze_expr(&args[0])?;
                    let val = self.analyze_expr(&args[1])?;
                    // Wrap index in cast to usize
                    let idx_casted = self.create_expr(IrExprKind::Cast {
                        target: Box::new(idx),
                        ty: "usize".to_string(),
                    }, Type::Unknown);
                    Ok(Some(self.create_expr(IrExprKind::MethodCall {
                        target_type: Type::Unknown,
                        target: Box::new(target_ir.clone()),
                        method: "insert".to_string(),
                        args: vec![idx_casted, val],
                        callee_needs_bridge: false,
                    }, Type::Unit)))
                } else {
                    Ok(None) // dict.insert handled by default
                }
            }
            _ => Ok(None), // Not a special method, use default handling
        }
    }

    /// Simple IR expression to string conversion for use in closures
    pub(crate) fn emit_simple_expr(&self, expr: &IrExpr) -> String {
        match &expr.kind {
            IrExprKind::Var(name) => name.clone(),
            IrExprKind::IntLit(n) => format!("{n}i64"),
            IrExprKind::FloatLit(f) => format!("{f}"),
            IrExprKind::StringLit(s) => format!("\"{s}\""),
            IrExprKind::BoolLit(b) => b.to_string(),
            _ => "x".to_string(),
        }
    }

    /// Simple IR expression to string conversion
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn emit_simple_ir_expr(&self, expr: &IrExpr) -> String {
        match &expr.kind {
            IrExprKind::Var(name) => name.clone(),
            IrExprKind::IntLit(n) => format!("{n}i64"),
            IrExprKind::FloatLit(f) => format!("{f}"),
            IrExprKind::BoolLit(b) => b.to_string(),
            IrExprKind::StringLit(s) => format!("\"{s}\".to_string()"),
            IrExprKind::BuiltinCall { id, args } => {
                match id {
                    crate::ir::exprs::BuiltinId::Len if args.len() == 1 => {
                        let target_str = self.emit_simple_ir_expr(&args[0]);
                        format!("{target_str}.len()")
                    }
                    _ => "expr".to_string(),
                }
            }
            IrExprKind::MethodCall {
                target,
                method,
                args,
                ..
            } => {
                let target_str = self.emit_simple_ir_expr(target);
                if args.is_empty() {
                    format!("{target_str}.{method}()")
                } else {
                    let args_str: Vec<String> =
                        args.iter().map(|a| self.emit_simple_ir_expr(a)).collect();
                    format!("{target_str}.{method}({})", args_str.join(", "))
                }
            }
            IrExprKind::BinOp { op, left, right } => {
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
                    _ => "??",
                };
                format!(
                    "{} {} {}",
                    self.emit_simple_ir_expr(left),
                    op_str,
                    self.emit_simple_ir_expr(right)
                )
            }
            IrExprKind::RawCode(code) => code.clone(),
            _ => "expr".to_string(),
        }
    }

    /// Get expected parameter types for built-in methods
    pub(crate) fn get_method_param_types(&self, target_ty: &Type, method: &str) -> Vec<Type> {
        match (target_ty, method) {
            (Type::List(inner), "push") | (Type::List(inner), "append") => {
                vec![inner.as_ref().clone()]
            }
            (Type::List(inner), "extend") => {
                vec![Type::Ref(Box::new(Type::List(inner.clone())))]
            }
            _ => vec![],
        }
    }

    pub(crate) fn to_snake_case(&self, s: &str) -> String {
        let mut res = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                res.push('_');
            }
            res.push(c.to_lowercase().next().unwrap());
        }
        res
    }

    pub(crate) fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Struct(name) => {
                if let Some(info) = self.scope.lookup(name) {
                    // Prevent infinite recursion: if the resolved type is the same Struct, return it
                    if let Type::Struct(resolved_name) = &info.ty {
                        if resolved_name == name {
                            return ty.clone();
                        }
                    }
                    return self.resolve_type(&info.ty);
                }
                // Return the Struct type itself if not found (it's a legitimate struct type)
                ty.clone()
            }
            Type::Ref(inner) => self.resolve_type(inner),
            _ => ty.clone(),
        }
    }
}

// =============================================================================
// V1.5.2 Tests
// =============================================================================

#[cfg(test)]
mod v1_5_2_tests {
    use super::*;
    use crate::ir::BuiltinId;
    use crate::parser::*;
    use crate::semantic::SemanticAnalyzer;

    fn expr_has_method(expr: &IrExpr, name: &str) -> bool {
        match &expr.kind {
            IrExprKind::MethodCall { method, target, args, .. } => {
                method == name
                    || expr_has_method(target, name)
                    || args.iter().any(|a| expr_has_method(a, name))
            }
            IrExprKind::Call { func, args, .. } => {
                expr_has_method(func, name) || args.iter().any(|a| expr_has_method(a, name))
            }
            IrExprKind::BridgeMethodCall { method, target, args, .. } => {
                method == name
                    || expr_has_method(target, name)
                    || args.iter().any(|a| expr_has_method(a, name))
            }
            _ => false,
        }
    }

    // Test int(Any) resolves to BuiltinCall (conversion is handled in Lowering)
    #[test]
    fn test_int_any_generates_json_conversion() {
        // Create a variable of Type::Any and call int() on it
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("json_value", Type::Any, false);

        // Parse: int(json_value)
        let call_expr = Expr::Call {
            func: Box::new(Expr::Ident("int".to_string())),
            args: vec![Expr::Ident("json_value".to_string())],
            kwargs: vec![],
        };

        let result = analyzer.analyze_expr(&call_expr);
        assert!(result.is_ok());
        let ir = result.unwrap();

        match ir.kind {
            IrExprKind::BuiltinCall { id, .. } => {
                assert_eq!(id, BuiltinId::Int);
            }
            _ => {
                panic!("Expected BuiltinCall for int(Any), got {:?}", ir);
            }
        }
    }

    // Test int(i64) resolves to BuiltinCall (conversion is handled in Lowering)
    #[test]
    fn test_int_i64_generates_cast() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("int_value", Type::Int, false);

        let call_expr = Expr::Call {
            func: Box::new(Expr::Ident("int".to_string())),
            args: vec![Expr::Ident("int_value".to_string())],
            kwargs: vec![],
        };

        let result = analyzer.analyze_expr(&call_expr);
        assert!(result.is_ok());
        let ir = result.unwrap();

        match ir.kind {
            IrExprKind::BuiltinCall { id, .. } => {
                assert_eq!(id, BuiltinId::Int);
            }
            _ => {
                panic!("Expected BuiltinCall for int(i64), got {:?}", ir);
            }
        }
    }

    // Test float(Any) resolves to BuiltinCall (conversion is handled in Lowering)
    #[test]
    fn test_float_any_generates_json_conversion() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("json_value", Type::Any, false);

        let call_expr = Expr::Call {
            func: Box::new(Expr::Ident("float".to_string())),
            args: vec![Expr::Ident("json_value".to_string())],
            kwargs: vec![],
        };

        let result = analyzer.analyze_expr(&call_expr);
        assert!(result.is_ok());
        let ir = result.unwrap();

        match ir.kind {
            IrExprKind::BuiltinCall { id, .. } => {
                assert_eq!(id, BuiltinId::Float);
            }
            _ => {
                panic!("Expected BuiltinCall for float(Any), got {:?}", ir);
            }
        }
    }

    #[test]
    fn test_dict_keys_method_call() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("d", Type::Dict(Box::new(Type::Int), Box::new(Type::String)), false);
        let call_expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("d".to_string())),
                attr: "keys".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        let ir = analyzer.analyze_expr(&call_expr).unwrap();
        assert!(expr_has_method(&ir, "keys"));
    }

    #[test]
    fn test_dict_values_method_call() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("d", Type::Dict(Box::new(Type::Int), Box::new(Type::String)), false);
        let call_expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("d".to_string())),
                attr: "values".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        let ir = analyzer.analyze_expr(&call_expr).unwrap();
        assert!(expr_has_method(&ir, "values"));
    }

    #[test]
    fn test_list_append_method_call() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("xs", Type::List(Box::new(Type::Int)), true);
        let call_expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("xs".to_string())),
                attr: "append".to_string(),
            }),
            args: vec![Expr::IntLiteral(1)],
            kwargs: vec![],
        };
        let ir = analyzer.analyze_expr(&call_expr).unwrap();
        assert!(expr_has_method(&ir, "append") || expr_has_method(&ir, "push"));
    }

    #[test]
    fn test_dict_get_method_call() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("d", Type::Dict(Box::new(Type::String), Box::new(Type::Int)), false);
        let call_expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("d".to_string())),
                attr: "get".to_string(),
            }),
            args: vec![Expr::StringLiteral("a".to_string())],
            kwargs: vec![],
        };
        let ir = analyzer.analyze_expr(&call_expr).unwrap();
        assert!(expr_has_method(&ir, "get"));
    }

    #[test]
    fn test_list_insert_method_call() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("xs", Type::List(Box::new(Type::Int)), true);
        let call_expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("xs".to_string())),
                attr: "insert".to_string(),
            }),
            args: vec![Expr::IntLiteral(0), Expr::IntLiteral(1)],
            kwargs: vec![],
        };
        let ir = analyzer.analyze_expr(&call_expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::MethodCall { method, .. } if method == "insert"));
    }

    #[test]
    fn test_string_upper_method_call() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.scope.define("s", Type::String, false);
        let call_expr = Expr::Call {
            func: Box::new(Expr::Attribute {
                value: Box::new(Expr::Ident("s".to_string())),
                attr: "upper".to_string(),
            }),
            args: vec![],
            kwargs: vec![],
        };
        let ir = analyzer.analyze_expr(&call_expr).unwrap();
        assert!(matches!(ir.kind, IrExprKind::MethodCall { method, .. } if method == "upper"));
    }
}
