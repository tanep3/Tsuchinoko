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
        &self,
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
                    && matches!(ir_arg, IrExpr::StringLit(_))
                {
                    IrExpr::MethodCall {
                        target: Box::new(ir_arg),
                        method: "to_string".to_string(),
                        args: vec![],
                    }
                } else {
                    ir_arg
                };

                // Wrap the argument in Some()
                return IrExpr::Call {
                    func: Box::new(IrExpr::Var("Some".to_string())),
                    args: vec![wrapped_arg],
                    callee_may_raise: false,
                };
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
                ir_arg = IrExpr::BoxNew(Box::new(ir_arg));

                // If target was a named alias, add explicit cast
                if let Type::Struct(alias_name) = &target_ty {
                    ir_arg = IrExpr::Cast {
                        target: Box::new(ir_arg),
                        ty: alias_name.clone(),
                    };
                }
                return ir_arg;
            }
        }

        // 3. Auto-Ref for Index expressions
        if needs_ref && matches!(expr, Expr::Index { .. }) {
            return IrExpr::Reference {
                target: Box::new(ir_arg),
            };
        }

        // 3.5 Auto-MutRef for mutable reference parameters
        if needs_mut_ref {
            // Need a mutable reference
            return IrExpr::MutReference {
                target: Box::new(ir_arg),
            };
        }

        // 4. Auto-Ref/Deref logic
        if needs_ref {
            // Need a reference
            if let Type::Ref(_) = actual_ty {
                // Already a reference, use as-is
                ir_arg
            } else {
                // Not a reference, add one
                IrExpr::Reference {
                    target: Box::new(ir_arg),
                }
            }
        } else {
            // Need an owned value - apply Auto-Deref for Copy types
            let mut current_ty = actual_ty.clone();
            while let Type::Ref(inner) = &current_ty {
                let inner_ty = inner.as_ref();
                if inner_ty.is_copy() {
                    ir_arg = IrExpr::UnaryOp {
                        op: IrUnaryOp::Deref,
                        operand: Box::new(ir_arg),
                    };
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
                matches!(&ir_arg, IrExpr::MethodCall { method, .. } if method == "len");
            if !resolved_actual.is_copy()
                && !matches!(actual_ty, Type::Ref(_))
                && !matches!(resolved_actual, Type::Func { .. })
                && !is_copy_method
            {
                let method = if matches!(ir_arg, IrExpr::StringLit(_) | IrExpr::FString { .. }) {
                    "to_string"
                } else {
                    "clone"
                };
                ir_arg = IrExpr::MethodCall {
                    target: Box::new(ir_arg),
                    method: method.to_string(),
                    args: vec![],
                };
            }

            // Special case: &String -> String
            if let Type::Ref(inner) = actual_ty {
                if **inner == Type::String {
                    ir_arg = IrExpr::MethodCall {
                        target: Box::new(ir_arg),
                        method: "to_string".to_string(),
                        args: vec![],
                    };
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
        // Check for enumerate(iterable) or zip(a, b, ...)
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
                    let iter_call = IrExpr::MethodCall {
                        target: Box::new(ir_iterable),
                        method: "iter".to_string(),
                        args: vec![],
                    };
                    let enumerate_call = IrExpr::MethodCall {
                        target: Box::new(iter_call),
                        method: "enumerate".to_string(),
                        args: vec![],
                    };

                    // Add .map() to convert (usize, &T) -> (i64, T) with optional start offset
                    let map_closure = if let Some(start) = start_value {
                        format!("|(i, x)| (i as i64 + {start}, x.clone())")
                    } else {
                        "|(i, x)| (i as i64, x.clone())".to_string()
                    };
                    let mapped_call = IrExpr::MethodCall {
                        target: Box::new(enumerate_call),
                        method: "map".to_string(),
                        args: vec![IrExpr::RawCode(map_closure)],
                    };

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
                        let chars_call = IrExpr::MethodCall {
                            target: Box::new(ir_iterable),
                            method: "chars".to_string(),
                            args: vec![],
                        };
                        let rev_call = IrExpr::MethodCall {
                            target: Box::new(chars_call),
                            method: "rev".to_string(),
                            args: vec![],
                        };
                        return Ok((target.to_string(), rev_call, Type::Unknown));
                    }

                    let inner_elem_type = match &iterable_ty {
                        Type::List(elem) => elem.as_ref().clone(),
                        _ => Type::Unknown,
                    };

                    // Build: iterable.iter().rev().cloned()
                    let iter_call = IrExpr::MethodCall {
                        target: Box::new(ir_iterable),
                        method: "iter".to_string(),
                        args: vec![],
                    };
                    let rev_call = IrExpr::MethodCall {
                        target: Box::new(iter_call),
                        method: "rev".to_string(),
                        args: vec![],
                    };
                    let cloned_call = IrExpr::MethodCall {
                        target: Box::new(rev_call),
                        method: "cloned".to_string(),
                        args: vec![],
                    };

                    return Ok((target.to_string(), cloned_call, inner_elem_type));
                }

                // zip(a, b) -> a.iter().zip(b.iter()).map(|(x, y)| (x.clone(), y.clone()))
                if func_name == "zip" && args.len() >= 2 && kwargs.is_empty() {
                    let first_iterable = &args[0];
                    let ir_first = self.analyze_expr(first_iterable)?;
                    let first_ty = self.infer_type(first_iterable);

                    // Start with first.iter()
                    let mut ir_iter = IrExpr::MethodCall {
                        target: Box::new(ir_first),
                        method: "iter".to_string(),
                        args: vec![],
                    };

                    // Collect element types for tuple
                    let mut elem_types = vec![self.get_elem_type(&first_ty)];
                    let num_args = args.len();

                    // Chain .zip() for each additional iterable
                    for arg in args.iter().skip(1) {
                        let ir_arg = self.analyze_expr(arg)?;
                        let arg_ty = self.infer_type(arg);
                        elem_types.push(self.get_elem_type(&arg_ty));

                        // .zip(arg.iter())
                        let arg_iter = IrExpr::MethodCall {
                            target: Box::new(ir_arg),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        ir_iter = IrExpr::MethodCall {
                            target: Box::new(ir_iter),
                            method: "zip".to_string(),
                            args: vec![arg_iter],
                        };
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

                    ir_iter = IrExpr::MethodCall {
                        target: Box::new(ir_iter),
                        method: "map".to_string(),
                        args: vec![IrExpr::RawCode(map_closure)],
                    };

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
                ir_iter = IrExpr::MethodCall {
                    target: Box::new(ir_iter),
                    method: "iter".to_string(),
                    args: vec![],
                };
                ir_iter = IrExpr::MethodCall {
                    target: Box::new(ir_iter),
                    method: "cloned".to_string(),
                    args: vec![],
                };
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
                        let iter_call = IrExpr::MethodCall {
                            target: Box::new(target_ir.clone()),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        Ok(Some(IrExpr::MethodCall {
                            target: Box::new(iter_call),
                            method: "map".to_string(),
                            args: vec![IrExpr::RawCode("|(k, v)| (*k, v.clone())".to_string())],
                        }))
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
                    let keys_call = IrExpr::MethodCall {
                        target: Box::new(target_ir.clone()),
                        method: "keys".to_string(),
                        args: vec![],
                    };
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(keys_call),
                        method: "cloned".to_string(),
                        args: vec![],
                    }))
                }
                _ => Ok(None),
            },
            // V1.5.0: dict.values() -> dict.values().cloned()
            "values" if args.is_empty() => match _target_ty {
                Type::Dict(_, _) => {
                    let values_call = IrExpr::MethodCall {
                        target: Box::new(target_ir.clone()),
                        method: "values".to_string(),
                        args: vec![],
                    };
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(values_call),
                        method: "cloned".to_string(),
                        args: vec![],
                    }))
                }
                _ => Ok(None),
            },
            // V1.5.0: dict.get(k) -> dict.get(&k).cloned().unwrap()
            // V1.5.0: dict.get(k, default) -> dict.get(&k).cloned().unwrap_or(default)
            "get" if !args.is_empty() => {
                match _target_ty {
                    Type::Dict(_, _) => {
                        let key = self.analyze_expr(&args[0])?;
                        let get_call = IrExpr::MethodCall {
                            target: Box::new(target_ir.clone()),
                            method: "get".to_string(),
                            args: vec![IrExpr::Reference {
                                target: Box::new(key),
                            }],
                        };
                        let cloned_call = IrExpr::MethodCall {
                            target: Box::new(get_call),
                            method: "cloned".to_string(),
                            args: vec![],
                        };
                        if args.len() >= 2 {
                            // get(k, default) -> get(&k).cloned().unwrap_or(default)
                            let mut default = self.analyze_expr(&args[1])?;
                            // Convert string literals to String for type compatibility
                            if matches!(default, IrExpr::StringLit(_)) {
                                default = IrExpr::MethodCall {
                                    target: Box::new(default),
                                    method: "to_string".to_string(),
                                    args: vec![],
                                };
                            }
                            Ok(Some(IrExpr::MethodCall {
                                target: Box::new(cloned_call),
                                method: "unwrap_or".to_string(),
                                args: vec![default],
                            }))
                        } else {
                            // get(k) -> get(&k).cloned().unwrap()
                            Ok(Some(IrExpr::MethodCall {
                                target: Box::new(cloned_call),
                                method: "unwrap".to_string(),
                                args: vec![],
                            }))
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
                        let remove_call = IrExpr::MethodCall {
                            target: Box::new(target_ir.clone()),
                            method: "remove".to_string(),
                            args: vec![IrExpr::Reference {
                                target: Box::new(key),
                            }],
                        };
                        Ok(Some(IrExpr::MethodCall {
                            target: Box::new(remove_call),
                            method: "unwrap".to_string(),
                            args: vec![],
                        }))
                    }
                    _ => Ok(None), // list.pop is handled by emitter
                }
            }
            // V1.5.0: dict.update(other) -> dict.extend(other)
            "update" if args.len() == 1 => match _target_ty {
                Type::Dict(_, _) => {
                    let other = self.analyze_expr(&args[0])?;
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(target_ir.clone()),
                        method: "extend".to_string(),
                        args: vec![other],
                    }))
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
                    let iter_call = IrExpr::MethodCall {
                        target: Box::new(iterable_ir),
                        method: "iter".to_string(),
                        args: vec![],
                    };
                    let closure = IrExpr::Closure {
                        params: vec!["x".to_string()],
                        body: vec![IrNode::Expr(IrExpr::MethodCall {
                            target: Box::new(IrExpr::Var("x".to_string())),
                            method: "to_string".to_string(),
                            args: vec![],
                        })],
                        ret_type: Type::String,
                    };
                    let map_call = IrExpr::MethodCall {
                        target: Box::new(iter_call),
                        method: "map".to_string(),
                        args: vec![closure],
                    };
                    let collect_call = IrExpr::MethodCall {
                        target: Box::new(map_call),
                        method: "collect::<Vec<String>>".to_string(),
                        args: vec![],
                    };
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(collect_call),
                        method: "join".to_string(),
                        args: vec![target_ir.clone()],
                    }))
                } else {
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(iterable_ir),
                        method: "join".to_string(),
                        args: vec![target_ir.clone()],
                    }))
                }
            }
            // V1.3.0: list.index(x) -> list.iter().position(|e| *e == x).unwrap() as i64
            "index" if args.len() == 1 => {
                match _target_ty {
                    Type::List(_) | Type::Ref(_) => {
                        let search_val = self.analyze_expr(&args[0])?;
                        // list.iter().position(|e| *e == val).unwrap() as i64
                        let iter_call = IrExpr::MethodCall {
                            target: Box::new(target_ir.clone()),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        let position_call = IrExpr::MethodCall {
                            target: Box::new(iter_call),
                            method: "position".to_string(),
                            args: vec![IrExpr::RawCode(format!(
                                "|e| *e == {}",
                                self.emit_simple_expr(&search_val)
                            ))],
                        };
                        let unwrap_call = IrExpr::MethodCall {
                            target: Box::new(position_call),
                            method: "unwrap".to_string(),
                            args: vec![],
                        };
                        // Cast to i64 (wrap in parens for correct precedence)
                        Ok(Some(IrExpr::RawCode(format!(
                            "({} as i64)",
                            self.emit_simple_ir_expr(&unwrap_call)
                        ))))
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
                        let iter_call = IrExpr::MethodCall {
                            target: Box::new(target_ir.clone()),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        let filter_call = IrExpr::MethodCall {
                            target: Box::new(iter_call),
                            method: "filter".to_string(),
                            args: vec![IrExpr::RawCode(format!(
                                "|e| **e == {}",
                                self.emit_simple_expr(&search_val)
                            ))],
                        };
                        let count_call = IrExpr::MethodCall {
                            target: Box::new(filter_call),
                            method: "count".to_string(),
                            args: vec![],
                        };
                        // Cast to i64 (wrap in parens for correct precedence)
                        Ok(Some(IrExpr::RawCode(format!(
                            "({} as i64)",
                            self.emit_simple_ir_expr(&count_call)
                        ))))
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
                    // Rust: let pos = list.iter().position(|e| *e == x).unwrap(); list.remove(pos);
                    // Since this is a statement expression, we generate:
                    // list.remove(list.iter().position(|e| *e == val).unwrap())
                    let iter_call = IrExpr::MethodCall {
                        target: Box::new(target_ir.clone()),
                        method: "iter".to_string(),
                        args: vec![],
                    };
                    let position_call = IrExpr::MethodCall {
                        target: Box::new(iter_call),
                        method: "position".to_string(),
                        args: vec![IrExpr::RawCode(format!(
                            "|e| *e == {}",
                            self.emit_simple_expr(&search_val)
                        ))],
                    };
                    let unwrap_call = IrExpr::MethodCall {
                        target: Box::new(position_call),
                        method: "unwrap".to_string(),
                        args: vec![],
                    };
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(target_ir.clone()),
                        method: "remove".to_string(),
                        args: vec![unwrap_call],
                    }))
                } else {
                    // Check if this is a set type
                    let is_set = matches!(_target_ty, Type::Set(_))
                        || matches!(_target_ty, Type::Ref(inner) if matches!(inner.as_ref(), Type::Set(_)));

                    if is_set {
                        // Python set.remove(x) -> Rust set.remove(&x)
                        let search_val = self.analyze_expr(&args[0])?;
                        Ok(Some(IrExpr::MethodCall {
                            target: Box::new(target_ir.clone()),
                            method: "remove".to_string(),
                            args: vec![IrExpr::Reference {
                                target: Box::new(search_val),
                            }],
                        }))
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
                    let idx_casted = IrExpr::Cast {
                        target: Box::new(idx),
                        ty: "usize".to_string(),
                    };
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(target_ir.clone()),
                        method: "insert".to_string(),
                        args: vec![idx_casted, val],
                    }))
                } else {
                    Ok(None) // dict.insert handled by default
                }
            }
            _ => Ok(None), // Not a special method, use default handling
        }
    }

    /// Simple IR expression to string conversion for use in closures
    pub(crate) fn emit_simple_expr(&self, expr: &IrExpr) -> String {
        match expr {
            IrExpr::Var(name) => name.clone(),
            IrExpr::IntLit(n) => format!("{n}i64"),
            IrExpr::FloatLit(f) => format!("{f}"),
            IrExpr::StringLit(s) => format!("\"{s}\""),
            IrExpr::BoolLit(b) => b.to_string(),
            _ => "x".to_string(),
        }
    }

    /// Simple IR expression to string conversion
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn emit_simple_ir_expr(&self, expr: &IrExpr) -> String {
        match expr {
            IrExpr::Var(name) => name.clone(),
            IrExpr::IntLit(n) => format!("{n}i64"),
            IrExpr::FloatLit(f) => format!("{f}"),
            IrExpr::BoolLit(b) => b.to_string(),
            IrExpr::StringLit(s) => format!("\"{s}\".to_string()"),
            IrExpr::MethodCall {
                target,
                method,
                args,
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
            IrExpr::BinOp { op, left, right } => {
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
            IrExpr::RawCode(code) => code.clone(),
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

    /// Handle built-in function calls (range, len, list, str, tuple, dict, max)
    /// Returns Some(IrExpr) if handled, None if not a built-in
    pub(crate) fn try_handle_builtin_call(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<Option<IrExpr>, TsuchinokoError> {
        match (name, args.len()) {
            // V1.3.1: int(x) -> x as i64 (handled here to avoid emitter responsibility)
            // V1.5.2: If x is Type::Any (serde_json::Value), use JsonConversion
            ("int", 1) => {
                let arg_ty = self.infer_type(&args[0]);
                let arg = self.analyze_expr(&args[0])?;
                if matches!(arg_ty, Type::Any) {
                    Ok(Some(IrExpr::JsonConversion {
                        target: Box::new(arg),
                        convert_to: "i64".to_string(),
                    }))
                } else {
                    Ok(Some(IrExpr::Cast {
                        target: Box::new(arg),
                        ty: "i64".to_string(),
                    }))
                }
            }
            // V1.3.1: float(x) -> x as f64 (handled here to avoid emitter responsibility)
            // V1.5.2: If x is Type::Any (serde_json::Value), use JsonConversion
            ("float", 1) => {
                let arg_ty = self.infer_type(&args[0]);
                let arg = self.analyze_expr(&args[0])?;
                if matches!(arg_ty, Type::Any) {
                    Ok(Some(IrExpr::JsonConversion {
                        target: Box::new(arg),
                        convert_to: "f64".to_string(),
                    }))
                } else {
                    Ok(Some(IrExpr::Cast {
                        target: Box::new(arg),
                        ty: "f64".to_string(),
                    }))
                }
            }
            ("range", 1) => {
                let start = IrExpr::IntLit(0);
                let end = self.analyze_expr(&args[0])?;
                Ok(Some(IrExpr::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                }))
            }
            ("range", 2) => {
                let start = self.analyze_expr(&args[0])?;
                let end = self.analyze_expr(&args[1])?;
                Ok(Some(IrExpr::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                }))
            }
            ("len", 1) => {
                let arg_ty = self.infer_type(&args[0]);
                let arg = self.analyze_expr(&args[0])?;
                // V1.5.0: If arg is Optional, unwrap first
                let arg = if matches!(arg_ty, Type::Optional(_)) {
                    IrExpr::MethodCall {
                        target: Box::new(arg),
                        method: "unwrap".to_string(),
                        args: vec![],
                    }
                } else {
                    arg
                };
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(arg),
                    method: "len".to_string(),
                    args: vec![],
                }))
            }
            ("list", 1) => {
                // list(iterable) -> iterable.collect::<Vec<_>>()
                // Special case: list(dict.items()) needs iter().map(|(k,v)| (*k, v.clone())).collect()

                // Check if arg is a method call to .items() on a dict
                if let Expr::Call {
                    func,
                    args: call_args,
                    ..
                } = &args[0]
                {
                    if call_args.is_empty() {
                        if let Expr::Attribute {
                            value: item_target,
                            attr,
                        } = func.as_ref()
                        {
                            if attr == "items" {
                                let target_ty = self.infer_type(item_target);
                                if matches!(target_ty, Type::Dict(_, _)) {
                                    // This is list(dict.items()) - generate iter().map(clone).collect()
                                    let ir_target = self.analyze_expr(item_target)?;
                                    let iter_call = IrExpr::MethodCall {
                                        target: Box::new(ir_target),
                                        method: "iter".to_string(),
                                        args: vec![],
                                    };
                                    let map_call = IrExpr::MethodCall {
                                        target: Box::new(iter_call),
                                        method: "map".to_string(),
                                        args: vec![IrExpr::RawCode(
                                            "|(k, v)| (*k, v.clone())".to_string(),
                                        )],
                                    };
                                    return Ok(Some(IrExpr::MethodCall {
                                        target: Box::new(map_call),
                                        method: "collect::<Vec<_>>".to_string(),
                                        args: vec![],
                                    }));
                                }
                            }
                        }
                    }
                }

                let arg = self.analyze_expr(&args[0])?;

                // If arg is a MethodCall (likely an iterator like .iter())
                // or a GenExpr/ListComp, always add collect()
                if matches!(arg, IrExpr::MethodCall { .. } | IrExpr::ListComp { .. }) {
                    return Ok(Some(IrExpr::MethodCall {
                        target: Box::new(arg),
                        method: "collect::<Vec<_>>".to_string(),
                        args: vec![],
                    }));
                }
                // Otherwise, use .to_vec() to convert slice/vec to owned Vec
                // This handles list(some_slice) -> some_slice.to_vec()
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(arg),
                    method: "to_vec".to_string(),
                    args: vec![],
                }))
            }
            ("str", 1) => {
                let arg = self.analyze_expr(&args[0])?;
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(arg),
                    method: "to_string".to_string(),
                    args: vec![],
                }))
            }
            ("tuple", 1) => {
                let ir_arg = self.analyze_expr(&args[0])?;
                // If already an iterator/collection producing IR, don't wrap
                if matches!(ir_arg, IrExpr::ListComp { .. } | IrExpr::MethodCall { .. }) {
                    return Ok(Some(ir_arg));
                }
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(ir_arg),
                    method: "collect::<Vec<_>>".to_string(),
                    args: vec![],
                }))
            }
            ("dict", 1) => {
                // dict(x) -> x.clone()
                // Python dict(some_dict) creates a copy, Rust .clone() does the same
                let ir_arg = self.analyze_expr(&args[0])?;
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(ir_arg),
                    method: "clone".to_string(),
                    args: vec![],
                }))
            }
            ("max", 1) => {
                let arg = self.analyze_expr(&args[0])?;
                let iter_call = IrExpr::MethodCall {
                    target: Box::new(arg),
                    method: "iter".to_string(),
                    args: vec![],
                };
                let max_call = IrExpr::MethodCall {
                    target: Box::new(iter_call),
                    method: "max".to_string(),
                    args: vec![],
                };
                let copied_call = IrExpr::MethodCall {
                    target: Box::new(max_call),
                    method: "cloned".to_string(),
                    args: vec![],
                };
                let unwrap_call = IrExpr::MethodCall {
                    target: Box::new(copied_call),
                    method: "unwrap".to_string(),
                    args: vec![],
                };
                Ok(Some(unwrap_call))
            }
            _ => Ok(None),
        }
    }
}

// =============================================================================
// V1.5.2 Tests
// =============================================================================

#[cfg(test)]
mod v1_5_2_tests {
    use super::*;
    use crate::parser::*;
    use crate::semantic::SemanticAnalyzer;

    // Test int(Any) generates JsonConversion
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
        
        // Should generate JsonConversion, not Cast
        match ir {
            IrExpr::JsonConversion { convert_to, .. } => {
                assert_eq!(convert_to, "i64");
            }
            IrExpr::Cast { .. } => {
                panic!("Expected JsonConversion for int(Any), got Cast");
            }
            _ => {
                panic!("Expected JsonConversion, got {:?}", ir);
            }
        }
    }

    // Test int(i64) generates Cast (not JsonConversion)
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
        
        match ir {
            IrExpr::Cast { ty, .. } => {
                assert_eq!(ty, "i64");
            }
            _ => {
                panic!("Expected Cast for int(i64), got {:?}", ir);
            }
        }
    }

    // Test float(Any) generates JsonConversion
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
        
        match ir {
            IrExpr::JsonConversion { convert_to, .. } => {
                assert_eq!(convert_to, "f64");
            }
            _ => {
                panic!("Expected JsonConversion for float(Any), got {:?}", ir);
            }
        }
    }
}
