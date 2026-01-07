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
                        let method_call = IrExpr::MethodCall {
                            target: Box::new(ir_left),
                            method: method.to_string(),
                            args: vec![IrExpr::Reference {
                                target: Box::new(ir_right),
                            }],
                        };
                        let cloned_call = IrExpr::MethodCall {
                            target: Box::new(method_call),
                            method: "cloned".to_string(),
                            args: vec![],
                        };
                        // Use collect_hashset marker for type inference
                        return Ok(IrExpr::MethodCall {
                            target: Box::new(cloned_call),
                            method: "collect_hashset".to_string(),
                            args: vec![],
                        });
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
                        let ir_right = if matches!(ir_right, IrExpr::StringLit(_)) {
                            IrExpr::MethodCall {
                                target: Box::new(ir_right),
                                method: "to_string".to_string(),
                                args: vec![],
                            }
                        } else {
                            ir_right
                        };
                        // x.unwrap_or(default)
                        return Ok(IrExpr::MethodCall {
                            target: Box::new(ir_left),
                            method: "unwrap_or".to_string(),
                            args: vec![ir_right],
                        });
                    }
                }

                // V1.5.0: 'or' with empty String falsy behavior
                // x or default -> if x.is_empty() { default } else { x }
                if matches!(op, AstBinOp::Or) {
                    let left_ty = self.infer_type(left);
                    if matches!(left_ty, Type::String) {
                        let ir_left = self.analyze_expr(left)?;
                        let ir_right = self.analyze_expr(right)?;
                        let left_str = self.emit_simple_ir_expr(&ir_left);
                        let right_str = self.emit_simple_ir_expr(&ir_right);
                        // if x.is_empty() { default } else { x.clone() }
                        return Ok(IrExpr::RawCode(format!(
                            "if {left_str}.is_empty() {{ {right_str} }} else {{ {left_str}.clone() }}"
                        )));
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
                            // Phase E: Register lambda param in scope before analyzing body
                            self.scope.push();
                            self.scope.define(&param, Type::Unknown, false);
                            let ir_body = self.analyze_expr(body)?;
                            self.scope.pop();
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

                    // V1.5.0: input() or input(prompt)
                    if name == "input" && kwargs.is_empty() {
                        if args.is_empty() {
                            // input() -> { let mut line = String::new(); std::io::stdin().read_line(&mut line).unwrap(); line.trim().to_string() }
                            return Ok(IrExpr::RawCode(
                                "{ let mut line = String::new(); std::io::stdin().read_line(&mut line).unwrap(); line.trim().to_string() }".to_string()
                            ));
                        } else {
                            // input(prompt) -> { print!("{}", prompt); flush; read_line; trim }
                            let ir_arg = self.analyze_expr(&args[0])?;
                            return Ok(IrExpr::RawCode(format!(
                                "{{ print!(\"{{}}\", {}); std::io::Write::flush(&mut std::io::stdout()).unwrap(); let mut line = String::new(); std::io::stdin().read_line(&mut line).unwrap(); line.trim().to_string() }}",
                                self.emit_simple_ir_expr(&ir_arg)
                            )));
                        }
                    }

                    // V1.5.0: round(x) or round(x, n)
                    if name == "round" && !args.is_empty() && kwargs.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;
                        if args.len() >= 2 {
                            // round(x, n) -> (x * 10^n).round() / 10^n
                            let ir_n = self.analyze_expr(&args[1])?;
                            return Ok(IrExpr::RawCode(format!(
                                "{{ let n = {}; let factor = 10f64.powi(n as i32); ({} * factor).round() / factor }}",
                                self.emit_simple_ir_expr(&ir_n),
                                self.emit_simple_ir_expr(&ir_arg)
                            )));
                        } else {
                            // round(x) -> x.round() as i64
                            return Ok(IrExpr::RawCode(format!(
                                "{}.round() as i64",
                                self.emit_simple_ir_expr(&ir_arg)
                            )));
                        }
                    }

                    // V1.5.0: chr(n) -> char::from_u32(n as u32).unwrap().to_string()
                    if name == "chr" && args.len() == 1 && kwargs.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;
                        return Ok(IrExpr::RawCode(format!(
                            "char::from_u32({} as u32).unwrap().to_string()",
                            self.emit_simple_ir_expr(&ir_arg)
                        )));
                    }

                    // V1.5.0: ord(c) -> c.chars().next().unwrap() as i64
                    if name == "ord" && args.len() == 1 && kwargs.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;
                        return Ok(IrExpr::RawCode(format!(
                            "{}.chars().next().unwrap() as i64",
                            self.emit_simple_ir_expr(&ir_arg)
                        )));
                    }

                    // V1.5.0: bin(x) -> format!("0b{:b}", x)
                    if name == "bin" && args.len() == 1 && kwargs.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;
                        return Ok(IrExpr::RawCode(format!(
                            "format!(\"0b{{:b}}\", {})",
                            self.emit_simple_ir_expr(&ir_arg)
                        )));
                    }

                    // V1.5.0: hex(x) -> format!("0x{:x}", x)
                    if name == "hex" && args.len() == 1 && kwargs.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;
                        return Ok(IrExpr::RawCode(format!(
                            "format!(\"0x{{:x}}\", {})",
                            self.emit_simple_ir_expr(&ir_arg)
                        )));
                    }

                    // V1.5.0: oct(x) -> format!("0o{:o}", x)
                    if name == "oct" && args.len() == 1 && kwargs.is_empty() {
                        let ir_arg = self.analyze_expr(&args[0])?;
                        return Ok(IrExpr::RawCode(format!(
                            "format!(\"0o{{:o}}\", {})",
                            self.emit_simple_ir_expr(&ir_arg)
                        )));
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
                                                // Phase E: Register lambda param in scope before analyzing body
                                                self.scope.push();
                                                self.scope.define(param, Type::Unknown, false);
                                                let body_ir = self.analyze_expr(body)?;
                                                self.scope.pop();
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

                    // V1.5.0: set(iterable) -> iterable.iter().cloned().collect::<HashSet<_>>()
                    if name == "set" && args.len() == 1 && kwargs.is_empty() {
                        let arg = &args[0];
                        let ir_arg = self.analyze_expr(arg)?;

                        // Build: arg.iter().cloned().collect()
                        // Use MethodCall chain, emitter will handle turbofish
                        let iter_call = IrExpr::MethodCall {
                            target: Box::new(ir_arg),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        let cloned_call = IrExpr::MethodCall {
                            target: Box::new(iter_call),
                            method: "cloned".to_string(),
                            args: vec![],
                        };
                        // Special marker for set collect - emitter will add turbofish
                        return Ok(IrExpr::MethodCall {
                            target: Box::new(cloned_call),
                            method: "collect_hashset".to_string(), // Special marker
                            args: vec![],
                        });
                    }
                }

                // Handle PyO3 module calls: np.array(...) -> np.call_method1("array", (...))?
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Expr::Ident(module_alias) = value.as_ref() {
                        // Check if this is a PyO3 import alias
                        let is_pyo3_module = self
                            .external_imports
                            .iter()
                            .any(|(_, alias)| alias == module_alias);

                        if is_pyo3_module {
                            // Convert to PyO3 call
                            let ir_args: Vec<IrExpr> = args
                                .iter()
                                .map(|a| self.analyze_expr(a))
                                .collect::<Result<Vec<_>, _>>()?;

                            // V1.5.2: PyO3 calls can fail, mark current function as may_raise
                            self.current_func_may_raise = true;

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
                                    callee_may_raise: false,
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
                            Box::new(IrExpr::Var("main_py".to_string()))
                        } else {
                            Box::new(IrExpr::Var(name.clone()))
                        };

                        // V1.5.2: Check if callee may raise
                        let callee_may_raise = self.may_raise_funcs.contains(name);

                        Ok(IrExpr::Call {
                            func: final_func,
                            args: ir_args,
                            callee_may_raise,
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
                                            callee_may_raise: false,
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
                            // V1.5.2: PyO3 method calls can fail
                            self.current_func_may_raise = true;

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
                                    if let Type::Func { may_raise: true, .. } = &var_info.ty {
                                        callee_may_raise = true;
                                    }
                                }
                            }
                        }
                        
                        // Phase G: from-import functions always may raise
                        if let Expr::Ident(func_name) = func.as_ref() {
                            let is_from_import = self.external_imports.iter()
                                .any(|(_, item)| item == func_name);
                            if is_from_import {
                                callee_may_raise = true;
                            }
                        }
                        
                        // Propagate may_raise to current function
                        if callee_may_raise {
                            self.current_func_may_raise = true;
                        }
                        
                        let ir_func = self.analyze_expr(func)?;
                        Ok(IrExpr::Call {
                            func: Box::new(ir_func),
                            args: ir_args,
                            callee_may_raise,
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
                    // V1.5.2: PyO3 method calls can fail
                    self.current_func_may_raise = true;

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
                    IrExpr::MethodCall {
                        target: Box::new(inner),
                        method: "is_some".to_string(),
                        args: vec![],
                    }
                } else if matches!(test_ty, Type::List(_)) {
                    // List variable as condition -> !x.is_empty()
                    let inner = self.analyze_expr(test)?;
                    IrExpr::UnaryOp {
                        op: IrUnaryOp::Not,
                        operand: Box::new(IrExpr::MethodCall {
                            target: Box::new(inner),
                            method: "is_empty".to_string(),
                            args: vec![],
                        }),
                    }
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
                            ir_body = IrExpr::MethodCall {
                                target: Box::new(ir_body),
                                method: "unwrap".to_string(),
                                args: vec![],
                            };
                        }
                    }
                    // Also if body is Optional type, unwrap it
                    if matches!(self.infer_type(body), Type::Optional(_)) {
                        ir_body = IrExpr::MethodCall {
                            target: Box::new(ir_body),
                            method: "unwrap".to_string(),
                            args: vec![],
                        };
                    }
                }

                // V1.5.0: If test was "x is not None" and body is x, unwrap body
                if let Some(ref opt_var) = optional_var_in_test {
                    if let Expr::Ident(body_var) = body.as_ref() {
                        if body_var == opt_var {
                            ir_body = IrExpr::MethodCall {
                                target: Box::new(ir_body),
                                method: "unwrap".to_string(),
                                args: vec![],
                            };
                        }
                    }
                }

                // V1.5.0: If orelse is StringLit, add to_string()
                if matches!(ir_orelse, IrExpr::StringLit(_)) {
                    ir_orelse = IrExpr::MethodCall {
                        target: Box::new(ir_orelse),
                        method: "to_string".to_string(),
                        args: vec![],
                    };
                }

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

                Ok(IrExpr::Set {
                    elem_type,
                    elements: ir_elements,
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
                        let is_reverse = matches!(step_box.as_ref(), IrExpr::IntLit(-1));

                        if is_reverse {
                            // s[::-1] -> s.chars().rev().collect::<String>()
                            return Ok(IrExpr::RawCode(format!(
                                "{}.chars().rev().collect::<String>()",
                                target_str
                            )));
                        } else {
                            // s[::n] -> s.chars().step_by(n).collect::<String>()
                            return Ok(IrExpr::RawCode(format!(
                                "{}.chars().step_by({} as usize).collect::<String>()",
                                target_str, step_val_str
                            )));
                        }
                    }
                }

                Ok(IrExpr::Slice {
                    target: Box::new(ir_target),
                    start: ir_start,
                    end: ir_end,
                    step: ir_step,
                })
            }
            Expr::Attribute { value, attr } => {
                // V1.4.0: Check for native constant access (math.pi, math.e)
                if let Expr::Ident(module) = value.as_ref() {
                    if module == "math" {
                        match attr.as_str() {
                            "pi" => {
                                return Ok(IrExpr::RawCode("std::f64::consts::PI".to_string()));
                            }
                            "e" => {
                                return Ok(IrExpr::RawCode("std::f64::consts::E".to_string()));
                            }
                            "tau" => {
                                return Ok(IrExpr::RawCode("std::f64::consts::TAU".to_string()));
                            }
                            "inf" => {
                                return Ok(IrExpr::RawCode("f64::INFINITY".to_string()));
                            }
                            "nan" => {
                                return Ok(IrExpr::RawCode("f64::NAN".to_string()));
                            }
                            _ => {}
                        }
                    }
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
        assert!(matches!(ir, IrExpr::IntLit(42)));
    }

    #[test]
    fn test_analyze_expr_float() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::FloatLiteral(3.14);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        if let IrExpr::FloatLit(f) = ir {
            assert!((f - 3.14).abs() < 0.001);
        }
    }

    #[test]
    fn test_analyze_expr_string() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::StringLiteral("hello".to_string());
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir, IrExpr::StringLit(_)));
    }

    #[test]
    fn test_analyze_expr_bool() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::BoolLiteral(true);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir, IrExpr::BoolLit(true)));
    }

    #[test]
    fn test_analyze_expr_none() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::NoneLiteral;
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir, IrExpr::NoneLit));
    }

    #[test]
    fn test_analyze_expr_ident() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.define("x", Type::Int, false);
        let expr = Expr::Ident("x".to_string());
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir, IrExpr::Var(_)));
    }

    #[test]
    fn test_analyze_expr_list() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::List(vec![Expr::IntLiteral(1), Expr::IntLiteral(2)]);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        if let IrExpr::List { elements, .. } = ir {
            assert_eq!(elements.len(), 2);
        }
    }

    #[test]
    fn test_analyze_expr_tuple() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::Tuple(vec![Expr::IntLiteral(1), Expr::IntLiteral(2)]);
        let ir = analyzer.analyze_expr(&expr).unwrap();
        if let IrExpr::Tuple(elements) = ir {
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
        assert!(matches!(ir, IrExpr::BinOp { .. }));
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
        assert!(matches!(ir, IrExpr::BinOp { .. }));
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
        assert!(matches!(ir, IrExpr::BinOp { .. }));
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
        assert!(matches!(ir, IrExpr::BinOp { .. }));
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
        assert!(matches!(ir, IrExpr::BinOp { .. }));
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
        assert!(matches!(ir, IrExpr::BinOp { .. }));
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
        assert!(matches!(ir, IrExpr::BinOp { .. }));
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
        assert!(matches!(ir, IrExpr::BinOp { .. }));
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
        assert!(matches!(ir, IrExpr::UnaryOp { .. }));
    }

    #[test]
    fn test_analyze_expr_unary_not() {
        let mut analyzer = SemanticAnalyzer::new();
        let expr = Expr::UnaryOp {
            op: crate::parser::UnaryOp::Not,
            operand: Box::new(Expr::BoolLiteral(true)),
        };
        let ir = analyzer.analyze_expr(&expr).unwrap();
        assert!(matches!(ir, IrExpr::UnaryOp { .. }));
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
        assert!(matches!(ir, IrExpr::Dict { .. }));
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
        assert!(matches!(ir, IrExpr::FString { .. }));
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
        assert!(matches!(ir, IrExpr::Index { .. }));
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
        assert!(matches!(ir, IrExpr::IfExp { .. }));
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
