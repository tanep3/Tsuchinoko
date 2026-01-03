//! Expression analysis for SemanticAnalyzer
//!
//! Extracted from mod.rs for maintainability

use super::*;

impl SemanticAnalyzer {
    pub(crate) fn analyze_expr(&mut self, expr: &Expr) -> Result<IrExpr, TsuchinokoError> {
        match expr {
            Expr::IntLiteral(n) => Ok(IrExpr::IntLit(*n)),
            Expr::FloatLiteral(f) => Ok(IrExpr::FloatLit(*f)),
            Expr::StringLiteral(s) => Ok(IrExpr::StringLit(s.clone())),
            Expr::BoolLiteral(b) => Ok(IrExpr::BoolLit(*b)),
            Expr::NoneLiteral => Ok(IrExpr::NoneLit),
            Expr::Ident(name) => {
                // Check if this variable has a narrowed type
                // If the original type is Optional<T> but it's narrowed to T, we need to unwrap
                if let Some(original_info) = self.scope.lookup(name) {
                    let original_ty = original_info.ty.clone();
                    if let Some(narrowed_ty) = self.scope.get_effective_type(name) {
                        // If original is Optional<T> and narrowed is T, emit Unwrap
                        if let Type::Optional(inner) = &original_ty {
                            if *inner.as_ref() == narrowed_ty {
                                return Ok(IrExpr::Unwrap(Box::new(IrExpr::Var(name.clone()))));
                            }
                        }
                    }
                }
                Ok(IrExpr::Var(name.clone()))
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

                    // For contains/contains_key, we typically pass references in Rust
                    // But analyze_expr might already return Values or Refs.
                    // Tsuchinoko Helper usually handles reference if needed, or we rely on MethodCall emission logic
                    // Emitter doesn't auto-ref args for MethodCall.
                    // Standard Vec::contains takes &T.
                    // If ir_left is passed as is, and it's an integer, we might need &
                    // But if ir_left is a variable `i`, usage `weekends.contains(&i)` is expected?
                    // Let's rely on semantic analysis of args to pass by ref?
                    // No, analyze_expr returns Expr.
                    // We should wrap ir_left in Ref? or just emit it and hope user code matches?
                    // IrExpr has no explicit Ref operator yet?
                    // Step 3364 shows `UnaryOp` handling. `IrUnaryOp` has `Not`, `Neg`.
                    // Does it have `Ref`? Let's check nodes.rs or handle it.
                    // If not, maybe use generic emission.
                    // FizzBuzz5 output used `contains_key(&i)`. How?
                    // Ah, `FunctionCall` to `is_divisible` uses `(i)`.
                    // Wait, `weekends.contains_key(&i)` was in output.
                    // How did `&i` get there?
                    // Maybe `IrExpr::Ref` exists? I should check.
                    // Generate y.method(&x) or y.method(x) depending on left type
                    let left_ty = self.infer_type(left);
                    let arg = if matches!(left_ty, Type::Ref(_) | Type::String) {
                        // Already a reference type, don't add another &
                        ir_left
                    } else {
                        IrExpr::Reference {
                            target: Box::new(ir_left),
                        }
                    };

                    return Ok(IrExpr::MethodCall {
                        target: Box::new(ir_right),
                        method: method.to_string(),
                        args: vec![arg],
                    });
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

                    // Generate !y.method(&x) or !y.method(x) depending on left type
                    let left_ty = self.infer_type(left);
                    let arg = if matches!(left_ty, Type::Ref(_) | Type::String) {
                        // Already a reference type, don't add another &
                        ir_left
                    } else {
                        IrExpr::Reference {
                            target: Box::new(ir_left),
                        }
                    };
                    let contains_call = IrExpr::MethodCall {
                        target: Box::new(ir_right),
                        method: method.to_string(),
                        args: vec![arg],
                    };

                    return Ok(IrExpr::UnaryOp {
                        op: IrUnaryOp::Not,
                        operand: Box::new(contains_call),
                    });
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
                                return Ok(IrExpr::MethodCall {
                                    target: Box::new(ir_left),
                                    method: method.to_string(),
                                    args: vec![],
                                });
                            }
                            _ => {
                                // Non-Optional type: always true/false
                                // Use RawCode to include warning comment
                                let (value, warning) = if matches!(op, AstBinOp::Is) {
                                    ("false", "/* Warning: 'is None' on non-Optional type is always false */")
                                } else {
                                    ("true", "/* Warning: 'is not None' on non-Optional type is always true */")
                                };
                                return Ok(IrExpr::RawCode(format!("{warning} {value}")));
                            }
                        }
                    }
                }

                let ir_left = self.analyze_expr(left)?;
                let ir_right = self.analyze_expr(right)?;
                let ir_op = self.convert_binop(op);
                Ok(IrExpr::BinOp {
                    left: Box::new(ir_left),
                    op: ir_op,
                    right: Box::new(ir_right),
                })
            }
            Expr::Call { func, args, kwargs } => {
                // Handle print() calls with type information for proper formatting
                if let Expr::Ident(name) = func.as_ref() {
                    if name == "print" {
                        let typed_args: Result<Vec<(IrExpr, Type)>, TsuchinokoError> = args
                            .iter()
                            .map(|a| {
                                let ir_arg = self.analyze_expr(a)?;
                                let ty = self.infer_type(a);
                                Ok((ir_arg, ty))
                            })
                            .collect();
                        return Ok(IrExpr::Print { args: typed_args? });
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
                            let ir_body = self.analyze_expr(body)?;
                            let body_str = self.emit_simple_ir_expr(&ir_body);

                            if reverse {
                                return Ok(IrExpr::RawCode(format!(
                                    "{{ let mut v = {}.to_vec(); v.sort_by(|a, b| {{ let {} = b; {} }}.cmp(&{{ let {} = a; {} }})); v }}",
                                    self.emit_simple_ir_expr(&ir_arg),
                                    param, body_str, param, body_str
                                )));
                            } else {
                                return Ok(IrExpr::RawCode(format!(
                                    "{{ let mut v = {}.to_vec(); v.sort_by_key(|{}| {}); v }}",
                                    self.emit_simple_ir_expr(&ir_arg),
                                    param,
                                    body_str
                                )));
                            }
                        } else if reverse {
                            return Ok(IrExpr::RawCode(format!(
                                "{{ let mut v = {}.to_vec(); v.sort_by(|a, b| b.cmp(a)); v }}",
                                self.emit_simple_ir_expr(&ir_arg)
                            )));
                        } else {
                            return Ok(IrExpr::RawCode(format!(
                                "{{ let mut v = {}.to_vec(); v.sort(); v }}",
                                self.emit_simple_ir_expr(&ir_arg)
                            )));
                        }
                    }

                    // V1.3.0: sum(iterable) or sum(iterable, start)
                    if name == "sum" && !args.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;

                        // Determine element type for sum type annotation
                        let arg_ty = self.infer_type(&args[0]);
                        let elem_ty = match &arg_ty {
                            Type::List(inner) => *inner.clone(),
                            Type::Ref(inner) => {
                                if let Type::List(elem) = inner.as_ref() {
                                    *elem.clone()
                                } else {
                                    Type::Int
                                }
                            }
                            _ => Type::Int,
                        };
                        let sum_type = if matches!(elem_ty, Type::Float) {
                            "f64"
                        } else {
                            "i64"
                        };

                        // Check for start argument (second positional argument)
                        if args.len() > 1 {
                            let ir_start = self.analyze_expr(&args[1])?;
                            return Ok(IrExpr::RawCode(format!(
                                "{}.iter().sum::<{}>() + {}",
                                self.emit_simple_ir_expr(&ir_arg),
                                sum_type,
                                self.emit_simple_ir_expr(&ir_start)
                            )));
                        } else {
                            // Use RawCode with type annotation to avoid type inference issues
                            return Ok(IrExpr::RawCode(format!(
                                "{}.iter().sum::<{}>()",
                                self.emit_simple_ir_expr(&ir_arg),
                                sum_type
                            )));
                        }
                    }

                    // V1.3.0: all(iterable) -> iterable.iter().all(|x| *x)
                    if name == "all" && !args.is_empty() && kwargs.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;
                        let iter_call = IrExpr::MethodCall {
                            target: Box::new(ir_arg),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        return Ok(IrExpr::MethodCall {
                            target: Box::new(iter_call),
                            method: "all".to_string(),
                            args: vec![IrExpr::RawCode("|x| *x".to_string())],
                        });
                    }

                    // V1.3.0: any(iterable) -> iterable.iter().any(|x| *x)
                    if name == "any" && !args.is_empty() && kwargs.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;
                        let iter_call = IrExpr::MethodCall {
                            target: Box::new(ir_arg),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        return Ok(IrExpr::MethodCall {
                            target: Box::new(iter_call),
                            method: "any".to_string(),
                            args: vec![IrExpr::RawCode("|x| *x".to_string())],
                        });
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

                                    let iter_call = IrExpr::MethodCall {
                                        target: Box::new(ir_iter),
                                        method: "iter".to_string(),
                                        args: vec![],
                                    };
                                    let map_call = IrExpr::MethodCall {
                                        target: Box::new(iter_call),
                                        method: "map".to_string(),
                                        args: vec![ir_lambda],
                                    };
                                    return Ok(IrExpr::MethodCall {
                                        target: Box::new(map_call),
                                        method: "collect".to_string(),
                                        args: vec![],
                                    });
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
                                                let body_ir = self.analyze_expr(body)?;
                                                IrExpr::RawCode(format!(
                                                    "|&{}| {}",
                                                    param,
                                                    self.emit_simple_ir_expr(&body_ir)
                                                ))
                                            } else {
                                                self.analyze_expr(lambda)?
                                            }
                                        } else {
                                            self.analyze_expr(lambda)?
                                        };

                                    let iter_call = IrExpr::MethodCall {
                                        target: Box::new(ir_iter),
                                        method: "iter".to_string(),
                                        args: vec![],
                                    };
                                    // cloned() before filter to get owned values
                                    let cloned_call = IrExpr::MethodCall {
                                        target: Box::new(iter_call),
                                        method: "cloned".to_string(),
                                        args: vec![],
                                    };
                                    let filter_call = IrExpr::MethodCall {
                                        target: Box::new(cloned_call),
                                        method: "filter".to_string(),
                                        args: vec![filter_closure],
                                    };
                                    return Ok(IrExpr::MethodCall {
                                        target: Box::new(filter_call),
                                        method: "collect".to_string(),
                                        args: vec![],
                                    });
                                }
                            }
                        }
                    }
                }

                // Handle PyO3 module calls: np.array(...) -> np.call_method1("array", (...))?
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Expr::Ident(module_alias) = value.as_ref() {
                        // Check if this is a PyO3 import alias
                        let is_pyo3_module = self
                            .pyo3_imports
                            .iter()
                            .any(|(_, alias)| alias == module_alias);

                        if is_pyo3_module {
                            // Convert to PyO3 call
                            let ir_args: Vec<IrExpr> = args
                                .iter()
                                .map(|a| self.analyze_expr(a))
                                .collect::<Result<Vec<_>, _>>()?;

                            // Return structured PyO3 call
                            return Ok(IrExpr::PyO3Call {
                                module: module_alias.clone(),
                                method: attr.clone(),
                                args: ir_args,
                            });
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
                                return Ok(IrExpr::Call {
                                    func: Box::new(IrExpr::RawCode(format!(
                                        "{class_name}::{attr}"
                                    ))),
                                    args: ir_args,
                                });
                            };
                            return Ok(IrExpr::RawCode(format!(
                                "{class_name}::{attr}({args_str})"
                            )));
                        }
                    }
                }

                // Handle math module functions: math.sqrt(x) -> x.sqrt()
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Expr::Ident(module) = value.as_ref() {
                        if module == "math" && args.len() == 1 && kwargs.is_empty() {
                            // Map math module functions to Rust f64 methods
                            let method = match attr.as_str() {
                                "sqrt" => Some("sqrt"),
                                "sin" => Some("sin"),
                                "cos" => Some("cos"),
                                "tan" => Some("tan"),
                                "asin" => Some("asin"),
                                "acos" => Some("acos"),
                                "atan" => Some("atan"),
                                "exp" => Some("exp"),
                                "log" => Some("ln"), // Python log() = Rust ln()
                                "log10" => Some("log10"),
                                "log2" => Some("log2"),
                                "abs" => Some("abs"),
                                "floor" => Some("floor"),
                                "ceil" => Some("ceil"),
                                "round" => Some("round"),
                                _ => None,
                            };

                            if let Some(rust_method) = method {
                                let ir_arg = self.analyze_expr(&args[0])?;
                                return Ok(IrExpr::MethodCall {
                                    target: Box::new(ir_arg),
                                    method: rust_method.to_string(),
                                    args: vec![],
                                });
                            }
                        }
                        // math.pi, math.e - constants
                        if module == "math" && args.is_empty() && kwargs.is_empty() {
                            match attr.as_str() {
                                "pi" => {
                                    return Ok(IrExpr::RawCode("std::f64::consts::PI".to_string()))
                                }
                                "e" => {
                                    return Ok(IrExpr::RawCode("std::f64::consts::E".to_string()))
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if let Expr::Attribute { value: _, attr } = func.as_ref() {
                    if attr == "items" && args.is_empty() && kwargs.is_empty() {
                        // Convert .items() to .iter() for HashMap
                        // Unwrap matches structure of Expr::Attribute.
                        if let Expr::Attribute { value, .. } = *func.clone() {
                            return Ok(IrExpr::MethodCall {
                                target: Box::new(self.analyze_expr(&value)?),
                                method: "iter".to_string(),
                                args: vec![],
                            });
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
                            let fields: Vec<(String, IrExpr)> =
                                field_names.into_iter().zip(ir_args).collect();

                            return Ok(IrExpr::StructConstruct {
                                name: name.clone(),
                                fields,
                            });
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
                            Box::new(IrExpr::Var("user_main".to_string()))
                        } else {
                            Box::new(IrExpr::Var(name.clone()))
                        };

                        Ok(IrExpr::Call {
                            func: final_func,
                            args: ir_args,
                        })
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
                                if let Some(info) = self.scope.lookup(&field_lookup) {
                                    // Resolve type aliases (e.g., ConditionFunction -> Func)
                                    let resolved_ty = self.resolve_type(&info.ty);
                                    if let Type::Func { .. } = resolved_ty {
                                        // This is a Callable field - emit as Call not MethodCall
                                        let ir_args =
                                            self.analyze_call_args(args, &[], &field_lookup)?;
                                        return Ok(IrExpr::Call {
                                            func: Box::new(IrExpr::FieldAccess {
                                                target: Box::new(ir_target),
                                                field: stripped_attr.to_string(),
                                            }),
                                            args: ir_args,
                                        });
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
                            return Ok(IrExpr::PyO3MethodCall {
                                target: Box::new(ir_target),
                                method: method_name.to_string(),
                                args: ir_args,
                            });
                        }

                        Ok(IrExpr::MethodCall {
                            target: Box::new(ir_target),
                            method: method_name.to_string(),
                            args: ir_args,
                        })
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
                        let ir_func = self.analyze_expr(func)?;
                        Ok(IrExpr::Call {
                            func: Box::new(ir_func),
                            args: ir_args,
                        })
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
                Ok(IrExpr::List {
                    elem_type,
                    elements: ir_elements,
                })
            }
            Expr::Index { target, index } => {
                let ir_target = self.analyze_expr(target)?;
                let ir_index = self.analyze_expr(index)?;

                // For sequence indexing, ensure the index is cast to usize
                let target_ty = self.infer_type(target);
                if matches!(target_ty, Type::Any) {
                    return Ok(IrExpr::PyO3MethodCall {
                        target: Box::new(ir_target),
                        method: "__getitem__".to_string(),
                        args: vec![ir_index],
                    });
                }

                let mut current_target_ty = target_ty.clone();
                while let Type::Ref(inner) | Type::MutRef(inner) = current_target_ty {
                    current_target_ty = *inner;
                }

                let final_index = match current_target_ty {
                    Type::List(_) | Type::Tuple(_) | Type::String => IrExpr::Cast {
                        target: Box::new(ir_index),
                        ty: "usize".to_string(),
                    },
                    _ => ir_index,
                };

                Ok(IrExpr::Index {
                    target: Box::new(ir_target),
                    index: Box::new(final_index),
                })
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
                                        IrExpr::MethodCall {
                                            target: Box::new(ir_target),
                                            method: "items".to_string(),
                                            args: vec![],
                                        }
                                    }
                                    Type::Dict(_, _) => {
                                        // Dict - use iter().map() for owned values
                                        let iter_call = IrExpr::MethodCall {
                                            target: Box::new(ir_target),
                                            method: "iter".to_string(),
                                            args: vec![],
                                        };
                                        IrExpr::MethodCall {
                                            target: Box::new(iter_call),
                                            method: "map".to_string(),
                                            args: vec![IrExpr::RawCode(
                                                "|(k, v)| (*k, v.clone())".to_string(),
                                            )],
                                        }
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

                Ok(IrExpr::ListComp {
                    elt: Box::new(ir_elt),
                    target: target.clone(),
                    iter: Box::new(ir_iter),
                    condition: ir_condition,
                })
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

                            let iter_call = IrExpr::MethodCall {
                                target: Box::new(ir_first),
                                method: "iter".to_string(),
                                args: vec![],
                            };
                            let zip_call = IrExpr::MethodCall {
                                target: Box::new(iter_call),
                                method: "zip".to_string(),
                                args: vec![IrExpr::MethodCall {
                                    target: Box::new(ir_second),
                                    method: "iter".to_string(),
                                    args: vec![],
                                }],
                            };
                            let mapped = IrExpr::MethodCall {
                                target: Box::new(zip_call),
                                method: "map".to_string(),
                                args: vec![IrExpr::RawCode(
                                    "|(x, y)| (x.clone(), y.clone())".to_string(),
                                )],
                            };

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
                            let iter_call = IrExpr::MethodCall {
                                target: Box::new(ir_items),
                                method: "iter".to_string(),
                                args: vec![],
                            };
                            let enum_call = IrExpr::MethodCall {
                                target: Box::new(iter_call),
                                method: "enumerate".to_string(),
                                args: vec![],
                            };
                            let mapped = IrExpr::MethodCall {
                                target: Box::new(enum_call),
                                method: "map".to_string(),
                                args: vec![IrExpr::RawCode(
                                    "|(i, x)| (i as i64, x.clone())".to_string(),
                                )],
                            };

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

                let mut unwrapped_ty = iter_ty;
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

                Ok(IrExpr::DictComp {
                    key: Box::new(ir_key),
                    value: Box::new(ir_value),
                    target: target.clone(),
                    iter: Box::new(ir_iter),
                    condition: ir_condition,
                })
            }
            Expr::IfExp { test, body, orelse } => {
                let ir_test = self.analyze_expr(test)?;
                let ir_body = self.analyze_expr(body)?;
                let ir_orelse = self.analyze_expr(orelse)?;

                Ok(IrExpr::IfExp {
                    test: Box::new(ir_test),
                    body: Box::new(ir_body),
                    orelse: Box::new(ir_orelse),
                })
            }
            Expr::Tuple(elements) => {
                let mut ir_elements = Vec::new();
                for e in elements {
                    ir_elements.push(self.analyze_expr(e)?);
                }
                Ok(IrExpr::Tuple(ir_elements))
            }
            Expr::Dict(entries) => {
                let mut ir_entries = Vec::new();
                for (k, v) in entries {
                    let ir_key = self.analyze_expr(k)?;
                    let ir_value = self.analyze_expr(v)?;
                    let val_ty = self.infer_type(v);

                    // Auto-convert string literals in Dict to String (.to_string())
                    let final_val = if let Type::String = val_ty {
                        if let IrExpr::StringLit(_) = ir_value {
                            IrExpr::MethodCall {
                                target: Box::new(ir_value),
                                method: "to_string".to_string(),
                                args: vec![],
                            }
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

                Ok(IrExpr::Dict {
                    key_type: final_key_type,
                    value_type: final_value_type,
                    entries: ir_entries,
                })
            }
            Expr::FString { parts, values } => {
                let ir_values: Vec<IrExpr> = values
                    .iter()
                    .map(|v| self.analyze_expr(v))
                    .collect::<Result<_, _>>()?;
                Ok(IrExpr::FString {
                    parts: parts.clone(),
                    values: ir_values,
                })
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

                Ok(IrExpr::Closure {
                    params: params.clone(),
                    body: vec![IrNode::Return(Some(Box::new(ir_body)))],
                    ret_type: Type::Unknown,
                })
            }
            Expr::Starred(inner) => {
                // Starred expression (*expr) - analyze the inner expression
                // The caller context will determine how to handle the spread
                let ir_inner = self.analyze_expr(inner)?;
                // For now, just return the inner expression - the context handles spread
                Ok(ir_inner)
            }
            Expr::Slice { target, start, end } => {
                // Python slices: nums[:3], nums[-3:], nums[1:len(nums)-1]
                // Rust equivalents depend on the slice type
                let ir_target = self.analyze_expr(target)?;

                let ir_start = match start {
                    Some(s) => Some(Box::new(self.analyze_expr(s)?)),
                    None => None,
                };

                let ir_end = match end {
                    Some(e) => Some(Box::new(self.analyze_expr(e)?)),
                    None => None,
                };

                Ok(IrExpr::Slice {
                    target: Box::new(ir_target),
                    start: ir_start,
                    end: ir_end,
                })
            }
            Expr::Attribute { value, attr } => {
                // Standalone attribute access (not call)
                // Could be field access.
                let ir_target = self.analyze_expr(value)?;
                // Strip dunder prefix for Python private fields -> Rust struct field
                let rust_field = if attr.starts_with("__") && !attr.ends_with("__") {
                    attr.trim_start_matches("__").to_string()
                } else {
                    attr.clone()
                };
                Ok(IrExpr::FieldAccess {
                    target: Box::new(ir_target),
                    field: rust_field,
                })
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
                    Ok(IrExpr::UnaryOp {
                        op: ir_op,
                        operand: Box::new(ir_operand),
                    })
                }
            }
        }
    }
}
