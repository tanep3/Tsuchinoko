//! Emitter module - Rust code generation

use crate::ir::{HoistedVar, IrAugAssignOp, IrBinOp, IrExpr, IrNode, IrUnaryOp};
use crate::semantic::Type;
use std::collections::HashMap;

/// Emit Rust code from IR
pub fn emit(nodes: &[IrNode]) -> String {
    let mut emitter = RustEmitter::new();
    emitter.emit_nodes(nodes)
}

/// Code emitter trait - enables multiple output formats
/// Implementations: RustEmitter (default), could add DebugEmitter, etc.
pub trait CodeEmitter {
    /// Emit a single IR node
    fn emit_node(&mut self, node: &IrNode) -> String;

    /// Emit an IR expression
    fn emit_expr(&mut self, expr: &IrExpr) -> String;

    /// Emit multiple nodes
    fn emit_nodes(&mut self, nodes: &[IrNode]) -> String {
        nodes
            .iter()
            .map(|n| self.emit_node(n))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Rust code emitter - implements CodeEmitter for Rust output
pub struct RustEmitter {
    indent: usize,
    /// Map of struct name -> field names (in order)
    struct_defs: HashMap<String, Vec<String>>,
    /// Whether PyO3 is needed for this file
    uses_pyo3: bool,
    /// Whether resident Python process is needed
    needs_resident: bool,
    /// PyO3 imports: (module, alias) - e.g., ("numpy", "np")
    external_imports: Vec<(String, String)>,
    /// Functions that require py_bridge argument
    resident_functions: std::collections::HashSet<String>,
    /// Whether we are currently emitting inside a function wrapper that already has py_bridge
    is_inside_resident_func: bool,
    /// Current function's hoisted variables (need Option<T> pattern)
    current_hoisted_vars: Vec<HoistedVar>,
    /// Variables hoisted in current TryBlock that need unwrap() in else body
    try_hoisted_vars: Vec<String>,
    /// V1.5.2: Variables that shadow hoisted variables in current scope (e.g., for loop vars)
    shadowed_vars: Vec<String>,
    /// V1.5.2: Whether TsuchinokoError type is needed (any may_raise=true function)
    uses_tsuchinoko_error: bool,
    /// V1.5.2: Whether current function may raise (for Ok() wrapping)
    current_func_may_raise: bool,
    /// V1.5.2: Whether we are inside try body closure (? not allowed, use .unwrap())
    in_try_body: bool,
    /// V1.6.0: Map of struct name -> base class name (for composition)
    struct_bases: HashMap<String, String>,
}

/// Convert camelCase/PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

impl Default for RustEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl RustEmitter {
    pub fn new() -> Self {
        Self {
            indent: 0,
            struct_defs: HashMap::new(),
            uses_pyo3: false,
            needs_resident: false,
            external_imports: Vec::new(),
            resident_functions: std::collections::HashSet::new(),
            is_inside_resident_func: false,
            current_hoisted_vars: Vec::new(),
            try_hoisted_vars: Vec::new(),
            shadowed_vars: Vec::new(),
            uses_tsuchinoko_error: false,
            current_func_may_raise: false,
            in_try_body: false,
            struct_bases: HashMap::new(),
        }
    }

    pub fn emit_nodes(&mut self, nodes: &[IrNode]) -> String {
        // If we're at indent 0, this is a top-level call
        let is_top_level = self.indent == 0;

        // Pass 1: Collect all PyO3Import nodes first (top-level only)
        if is_top_level {
            for node in nodes {
                if let IrNode::PyO3Import {
                    module,
                    alias,
                    items,
                } = node
                {
                    self.uses_pyo3 = true;
                    if let Some(ref item_list) = items {
                        // "from module import a, b, c"
                        for item in item_list {
                            self.external_imports.push((module.clone(), item.clone()));
                        }
                    } else {
                        // "import module" or "import module as alias"
                        let effective_alias = alias.clone().unwrap_or_else(|| module.clone());
                        self.external_imports
                            .push((module.clone(), effective_alias));
                    }
                }
            }
        }

        // Pass 2: Emit all nodes
        let code: Vec<String> = nodes
            .iter()
            .map(|n| self.emit_node(n))
            .filter(|s| !s.is_empty())
            .collect();

        let body = code.join("\n");

        // Only add wrapper at top level
        if is_top_level {
            // V1.5.2: Prepend TsuchinokoError if needed
            let error_def = if self.uses_tsuchinoko_error {
                crate::bridge::tsuchinoko_error::TSUCHINOKO_ERROR_DEFINITION
            } else {
                ""
            };

            let final_body = if !error_def.is_empty() {
                format!("{}\n{}", error_def, body)
            } else {
                body
            };

            if self.needs_resident {
                // py_bridge ランタイムを挿入（常駐プロセス方式）
                self.emit_resident_wrapped(&final_body)
            } else if self.uses_pyo3 {
                // Native で全て対応できたが import があった場合
                // → PyO3 不要、シンプルな Rust コード
                final_body
            } else {
                final_body
            }
        } else {
            body
        }
    }

    /// Wrap the code with PyO3 setup (legacy, kept for reference)
    #[allow(dead_code)]
    fn emit_pyo3_wrapped(&self, body: &str) -> String {
        format!(
            r#"use pyo3::prelude::*;
use pyo3::types::PyList;

{body}

// Note: To run this code, add to Cargo.toml:
// [dependencies]
// pyo3 = {{ version = "0", features = ["auto-initialize"] }}
//
// Use: tnk --pyo3-version 0.27 input.py -p project
// Activate your venv before running: source venv/bin/activate
"#
        )
    }

    /// Wrap the code with py_bridge runtime (常駐プロセス方式)
    fn emit_resident_wrapped(&self, body: &str) -> String {
        format!(
            r#"use tsuchinoko::bridge::PythonBridge;

{body}

// Note: This code uses the PythonBridge for calling Python libraries.
// Make sure Python is installed and the required libraries are available.
// The Python worker process will be started automatically.
"#
        )
    }

    /// Generate PyO3-wrapped main function
    #[allow(dead_code)]
    fn emit_pyo3_main(&self, user_body: &str) -> String {
        // Generate import statements for each PyO3 module
        let imports: Vec<String> = self
            .external_imports
            .iter()
            .map(|(module, alias)| {
                format!(
                    "        let {alias} = py.import(\"{module}\").expect(\"Failed to import {module}\");"
                )
            })
            .collect();

        let imports_str = imports.join("\n");

        // Indent user body
        let indented_body: String = user_body
            .lines()
            .map(|line| {
                if line.trim().is_empty() {
                    String::new()
                } else {
                    format!("        {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"fn main() {{
    pyo3::prepare_freethreaded_python();
    pyo3::Python::with_gil(|py| {{
{imports_str}
{indented_body}
    }});
}}"#
        )
    }

    /// Generate main with py_bridge initialization (常駐プロセス方式)
    fn emit_resident_main(&self, user_body: &str) -> String {
        // Indent user body
        let indented_body: String = user_body
            .lines()
            .map(|line| {
                if line.trim().is_empty() {
                    String::new()
                } else {
                    format!("    {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // V1.5.2: Generate main with Result type for PyO3 ? error propagation
        format!(
            r#"fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let mut py_bridge = tsuchinoko::bridge::PythonBridge::new()
        .expect("Failed to start Python worker");
    
{indented_body}
    Ok(())
}}"#
        )
    }

    fn emit_node_internal(&mut self, node: &IrNode) -> String {
        let indent = "    ".repeat(self.indent);
        match node {
            IrNode::VarDecl {
                name,
                ty,
                mutable,
                init,
            } => {
                let snake_name = to_snake_case(name);

                // Check if this variable is hoisted (already declared as Option<T>)
                let is_hoisted = self.current_hoisted_vars.iter().any(|v| v.name == *name);

                if is_hoisted {
                    // Hoisted variable: emit assignment with Some()
                    match init {
                        Some(expr) => {
                            let expr_str = self.emit_expr_no_outer_parens(expr);
                            format!("{indent}{snake_name} = Some({expr_str});")
                        }
                        None => {
                            // No init: leave as None (already declared)
                            String::new()
                        }
                    }
                } else {
                    // Normal variable declaration
                    let mut_kw = if *mutable { "mut " } else { "" };
                    let ty_annotation = if ty.contains_unknown() {
                        "".to_string()
                    } else {
                        format!(": {}", ty.to_rust_string())
                    };

                    match init {
                        Some(expr) => {
                            // If assigning a string literal to a String variable, add .to_string()
                            let expr_str = if matches!(ty, Type::String)
                                && matches!(expr.as_ref(), IrExpr::StringLit(_))
                            {
                                if let IrExpr::StringLit(s) = expr.as_ref() {
                                    format!("\"{s}\".to_string()")
                                } else {
                                    self.emit_expr_no_outer_parens(expr)
                                }
                            } else if matches!(expr.as_ref(), IrExpr::Tuple(_)) {
                                // Keep parentheses for tuple literals
                                self.emit_expr(expr)
                            } else {
                                self.emit_expr_no_outer_parens(expr)
                            };
                            format!("{indent}let {mut_kw}{snake_name}{ty_annotation} = {expr_str};")
                        }
                        None => {
                            format!("{indent}let {mut_kw}{snake_name}{ty_annotation};")
                        }
                    }
                }
            }
            IrNode::Assign { target, value } => {
                let snake_name = to_snake_case(target);

                // Check if this target is a hoisted variable (needs Some() wrapper)
                let is_func_hoisted = self
                    .current_hoisted_vars
                    .iter()
                    .any(|v| to_snake_case(&v.name) == snake_name);
                let is_try_hoisted = self.try_hoisted_vars.contains(&snake_name);

                if is_func_hoisted || is_try_hoisted {
                    // Hoisted variable: wrap value in Some()
                    format!(
                        "{}{} = Some({});",
                        indent,
                        snake_name,
                        self.emit_expr(value)
                    )
                } else {
                    format!("{}{} = {};", indent, snake_name, self.emit_expr(value))
                }
            }
            IrNode::FieldAssign {
                target,
                field,
                value,
            } => {
                format!(
                    "{}{}.{} = {};",
                    indent,
                    self.emit_expr(target),
                    to_snake_case(field),
                    self.emit_expr(value)
                )
            }
            IrNode::IndexAssign {
                target,
                index,
                value,
            } => {
                format!(
                    "{}{}[{}] = {};",
                    indent,
                    self.emit_expr(target),
                    self.emit_expr(index),
                    self.emit_expr(value)
                )
            }
            IrNode::AugAssign { target, op, value } => {
                // V1.3.0: Handle **= specially as Rust doesn't have it
                if matches!(op, IrAugAssignOp::Pow) {
                    return format!(
                        "{}{} = ({} as i64).pow(({}) as u32);",
                        indent,
                        target,
                        target,
                        self.emit_expr(value)
                    );
                }

                let op_str = match op {
                    IrAugAssignOp::Add => "+=",
                    IrAugAssignOp::Sub => "-=",
                    IrAugAssignOp::Mul => "*=",
                    IrAugAssignOp::Div => "/=",
                    IrAugAssignOp::FloorDiv => "/=", // Rust doesn't have //=, use /= for i64
                    IrAugAssignOp::Mod => "%=",
                    // V1.3.0 additions
                    IrAugAssignOp::Pow => unreachable!(), // Handled above
                    IrAugAssignOp::BitAnd => "&=",
                    IrAugAssignOp::BitOr => "|=",
                    IrAugAssignOp::BitXor => "^=",
                    IrAugAssignOp::Shl => "<<=",
                    IrAugAssignOp::Shr => ">>=",
                };
                format!("{}{} {} {};", indent, target, op_str, self.emit_expr(value))
            }
            IrNode::MultiAssign { targets, value } => {
                let targets_str = targets.join(", ");
                format!("{}({}) = {};", indent, targets_str, self.emit_expr(value))
            }
            IrNode::MultiVarDecl { targets, value } => {
                let vars_str: Vec<_> = targets
                    .iter()
                    .map(|(n, _, m)| {
                        let mut_kw = if *m { "mut " } else { "" };
                        format!("{mut_kw}{n}")
                    })
                    .collect();

                let has_unknown = targets.iter().any(|(_, t, _)| t.contains_unknown());

                if has_unknown {
                    format!(
                        "{}let ({}) = {};",
                        indent,
                        vars_str.join(", "),
                        self.emit_expr(value)
                    )
                } else {
                    let types_str: Vec<_> =
                        targets.iter().map(|(_, t, _)| t.to_rust_string()).collect();

                    format!(
                        "{}let ({}) : ({}) = {};",
                        indent,
                        vars_str.join(", "),
                        types_str.join(", "),
                        self.emit_expr(value)
                    )
                }
            }
            IrNode::FuncDecl {
                name,
                params,
                ret,
                body,
                hoisted_vars,
                may_raise,
            } => {
                // Check if this is the auto-generated top-level function -> "fn main"
                if name == "__top_level__" {
                    self.indent += 1;

                    // needs_resident をバックアップ
                    let needs_resident_backup = self.needs_resident;
                    self.needs_resident = false;

                    // V1.5.2: __top_level__ (main) is never may_raise
                    let old_may_raise = self.current_func_may_raise;
                    self.current_func_may_raise = false;

                    let body_str = self.emit_nodes(body);

                    // Restore may_raise state
                    self.current_func_may_raise = old_may_raise;

                    // 関数内で resident 機能が使われたか
                    let func_needs_resident = self.needs_resident;

                    // グローバルステートを復元（OR演算）
                    self.needs_resident = needs_resident_backup || func_needs_resident;

                    self.indent -= 1;

                    if func_needs_resident || self.needs_resident {
                        // self.needs_resident is global state (might be set by previous nodes)
                        self.emit_resident_main(&body_str)
                    } else {
                        // V1.5.2: Wrap main in catch_unwind for panic diagnosis
                        format!(
                            r#"fn main() {{
    let result = std::panic::catch_unwind(|| {{
{body_str}
    }});
    if let Err(e) = result {{
        let msg = if let Some(s) = e.downcast_ref::<&str>() {{
            s.to_string()
        }} else if let Some(s) = e.downcast_ref::<String>() {{
            s.clone()
        }} else {{
            "Unknown panic".to_string()
        }};
        eprintln!("InternalError: {{}}", msg);
        std::process::exit(1);
    }}
}}"#
                        )
                    }
                } else {
                    let snake_name = if name == "main" {
                        // Rename user's 'main' to 'main_py' to avoid conflict with Rust's entry point
                        "main_py".to_string()
                    } else {
                        to_snake_case(name)
                    };

                    // needs_resident をバックアップして、この関数内での変化を追跡
                    let needs_resident_backup = self.needs_resident;
                    self.needs_resident = false;

                    // Set current hoisted variables for this function scope
                    let old_hoisted =
                        std::mem::replace(&mut self.current_hoisted_vars, hoisted_vars.clone());

                    self.indent += 1;

                    // Generate Option<T> declarations for hoisted variables
                    let hoisted_decls = if !hoisted_vars.is_empty() {
                        let inner_indent = "    ".repeat(self.indent);
                        hoisted_vars
                            .iter()
                            .map(|v| {
                                format!(
                                    "{}let mut {}: Option<{}> = None;",
                                    inner_indent,
                                    to_snake_case(&v.name),
                                    v.ty.to_rust_string()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        String::new()
                    };

                    // V1.5.2: Set may_raise flag for return statement wrapping
                    let old_may_raise = self.current_func_may_raise;
                    self.current_func_may_raise = *may_raise;

                    let body_str = self.emit_nodes(body);
                    self.indent -= 1;

                    // Restore previous hoisted vars and may_raise
                    self.current_hoisted_vars = old_hoisted;
                    self.current_func_may_raise = old_may_raise;

                    let func_needs_resident = self.needs_resident;

                    // グローバルステートを復元（OR演算）
                    self.needs_resident = needs_resident_backup || func_needs_resident;

                    // 通常のパラメータ
                    let mut params_str: Vec<_> = params
                        .iter()
                        .map(|(n, t)| format!("{}: {}", to_snake_case(n), t.to_rust_string()))
                        .collect();

                    // Hack: If return type is Unit but body has Return with value, force return type to Value
                    let has_value_return =
                        body.iter().any(|n| matches!(n, IrNode::Return(Some(_))));

                    let effective_ret = if *ret == Type::Unit && has_value_return {
                        &Type::Any // Will be emitted as serde_json::Value
                    } else {
                        ret
                    };

                    // Re-generate body if resident features are needed, to ensure correct args passing
                    let final_body_str = if func_needs_resident {
                        // Re-emit with flag set
                        self.indent += 1;
                        let backup_flag = self.is_inside_resident_func;
                        // ONLY set this if we are NOT in the special __top_level__ (fn main)
                        // Actually, this block is the "else" (non-__top_level__) path, so it's always true.
                        self.is_inside_resident_func = true;

                        // Phase F: Set may_raise for proper Ok() wrapping in Return statements
                        let backup_may_raise = self.current_func_may_raise;
                        self.current_func_may_raise = *may_raise || func_needs_resident;

                        // Reset needs_resident just in case, though we know it will become true
                        self.needs_resident = false;
                        let s = self.emit_nodes(body);

                        self.is_inside_resident_func = backup_flag;
                        self.current_func_may_raise = backup_may_raise;
                        self.indent -= 1;
                        s
                    } else {
                        body_str
                    };

                    // Prepend hoisted variable declarations to body
                    let final_body_str = if !hoisted_decls.is_empty() {
                        format!("{}\n{}", hoisted_decls, final_body_str)
                    } else {
                        final_body_str
                    };

                    // V1.5.2: Determine return type string based on may_raise or external calls
                    // External calls (func_needs_resident) also require Result type
                    let effective_may_raise = *may_raise || func_needs_resident;
                    let ret_str = if effective_may_raise {
                        // Mark that TsuchinokoError type is needed
                        self.uses_tsuchinoko_error = true;
                        // Result<T, TsuchinokoError> for functions that may raise
                        let inner_type = effective_ret.to_rust_string();
                        format!("Result<{}, TsuchinokoError>", inner_type)
                    } else {
                        effective_ret.to_rust_string()
                    };

                    if func_needs_resident {
                        params_str.insert(
                            0,
                            "py_bridge: &mut tsuchinoko::bridge::PythonBridge".to_string(),
                        );
                        // 関数を resident_functions セットに登録
                        self.resident_functions.insert(snake_name.clone());
                    }

                    // V1.5.2: Add implicit Ok(()) for may_raise functions that return Unit
                    let final_body_with_ok = if effective_may_raise && *effective_ret == Type::Unit
                    {
                        let inner_indent = "    ".repeat(self.indent + 1);
                        format!("{}\n{}Ok(())", final_body_str, inner_indent)
                    } else {
                        final_body_str
                    };

                    format!(
                        "{}fn {}({}) -> {} {{\n{}\n{}}}",
                        indent,
                        snake_name,
                        params_str.join(", "),
                        ret_str,
                        final_body_with_ok,
                        indent
                    )
                }
            }
            IrNode::If {
                cond,
                then_block,
                else_block,
            } => {
                self.indent += 1;
                let then_str = self.emit_nodes(then_block);
                self.indent -= 1;

                let else_str = match else_block {
                    Some(block) => {
                        self.indent += 1;
                        let s = self.emit_nodes(block);
                        self.indent -= 1;
                        format!(" else {{\n{s}\n{indent}}}")
                    }
                    None => String::new(),
                };

                format!(
                    "{}if {} {{\n{}\n{}}}{}",
                    indent,
                    self.emit_expr_no_outer_parens(cond),
                    then_str,
                    indent,
                    else_str
                )
            }
            IrNode::For {
                var,
                var_type: _,
                iter,
                body,
            } => {
                // V1.5.2: Check which loop variables are hoisted
                let loop_vars: Vec<String> = if var.contains(',') {
                    var.split(',').map(|s| to_snake_case(s.trim())).collect()
                } else {
                    vec![to_snake_case(var)]
                };

                // Build mapping: (original_name, actual_loop_var_name, is_hoisted)
                // If hoisted, use _loop_<name> as loop variable to avoid shadowing
                let mut var_mapping: Vec<(String, String, bool)> = Vec::new();
                for lv in &loop_vars {
                    let is_hoisted = self
                        .current_hoisted_vars
                        .iter()
                        .any(|v| to_snake_case(&v.name) == *lv);
                    let loop_var_name = if is_hoisted {
                        format!("_loop_{}", lv)
                    } else {
                        lv.clone()
                    };
                    var_mapping.push((lv.clone(), loop_var_name, is_hoisted));
                }

                // Add renamed loop vars to shadowed_vars (these override hoisted check)
                let old_shadowed_len = self.shadowed_vars.len();
                for (_, loop_var_name, _) in &var_mapping {
                    self.shadowed_vars.push(loop_var_name.clone());
                }

                self.indent += 1;
                let body_str = self.emit_nodes(body);

                // V1.5.2: For hoisted loop variables, add assignment at START of loop body
                // This ensures i.unwrap() works inside the loop body
                let mut hoisted_init_assignments = Vec::new();
                for (hoisted_name, loop_var_name, is_hoisted) in &var_mapping {
                    if *is_hoisted {
                        let inner_indent = "    ".repeat(self.indent);
                        hoisted_init_assignments.push(format!(
                            "{}{} = Some({});",
                            inner_indent, hoisted_name, loop_var_name
                        ));
                    }
                }

                self.indent -= 1;

                // Restore shadowed_vars
                self.shadowed_vars.truncate(old_shadowed_len);

                // Build the loop variable string for the for statement
                let var_str = if var.contains(',') {
                    let parts: Vec<String> =
                        var_mapping.iter().map(|(_, lv, _)| lv.clone()).collect();
                    format!("({})", parts.join(", "))
                } else {
                    var_mapping[0].1.clone()
                };

                // Build final body: hoisted init + original body
                let final_body = if hoisted_init_assignments.is_empty() {
                    body_str
                } else {
                    format!("{}\n{}", hoisted_init_assignments.join("\n"), body_str)
                };

                format!(
                    "{}for {} in {} {{\n{}\n{}}}",
                    indent,
                    var_str,
                    self.emit_expr(iter),
                    final_body,
                    indent
                )
            }
            IrNode::While { cond, body } => {
                self.indent += 1;
                let body_str = self.emit_nodes(body);
                self.indent -= 1;

                format!(
                    "{}while {} {{\n{}\n{}}}",
                    indent,
                    self.emit_expr_no_outer_parens(cond),
                    body_str,
                    indent
                )
            }
            IrNode::Return(expr) => {
                // V1.5.2: Wrap in Ok() if current function may raise
                if self.current_func_may_raise {
                    match expr {
                        Some(e) => format!("{}return Ok({});", indent, self.emit_expr(e)),
                        None => format!("{indent}return Ok(());"),
                    }
                } else {
                    match expr {
                        Some(e) => format!("{}return {};", indent, self.emit_expr(e)),
                        None => format!("{indent}return;"),
                    }
                }
            }
            IrNode::TypeAlias { name, ty } => {
                format!("{}type {} = {};", indent, name, ty.to_rust_string())
            }
            IrNode::Expr(expr) => {
                // Convert standalone string literals (docstrings) to comments
                if let IrExpr::StringLit(s) = expr {
                    // Multi-line docstrings become multi-line comments
                    let comment_lines: Vec<String> =
                        s.lines().map(|line| format!("{indent}// {line}")).collect();
                    return comment_lines.join("\n");
                }
                format!("{}{};\n", indent, self.emit_expr(expr))
            }
            IrNode::StructDef { name, fields, base } => {
                // Register struct definition for constructor emission
                let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                self.struct_defs.insert(name.clone(), field_names);

                // V1.6.0: Track base class for constructor generation
                if let Some(base_name) = base {
                    self.struct_bases.insert(name.clone(), base_name.clone());
                }

                let mut result = format!("{indent}#[derive(Clone)]\n");
                result.push_str(&format!("{indent}struct {name} {{\n"));
                for (field_name, field_type) in fields {
                    let rust_type = field_type.to_rust_string();
                    result.push_str(&format!(
                        "{}    {}: {},\n",
                        indent,
                        to_snake_case(field_name),
                        rust_type
                    ));
                }
                result.push_str(&format!("{indent}}}"));
                result
            }
            IrNode::TryBlock {
                try_body,
                except_body,
                except_var, // V1.5.2
                else_body,  // V1.5.2
                finally_body,
            } => {
                let mut result = String::new();

                // V1.5.2: Always hoist variables from try_body
                // Variables defined in try may be used in except/else/finally
                let mut hoisted_vars: Vec<(String, Type)> = Vec::new();

                // Collect variable declarations from try_body
                for node in try_body.iter() {
                    if let IrNode::VarDecl { name, ty, .. } = node {
                        hoisted_vars.push((name.clone(), ty.clone()));
                    }
                }

                let need_hoisting = !hoisted_vars.is_empty();

                // Emit hoisted variable declarations as Option<T>
                for (name, ty) in &hoisted_vars {
                    let snake_name = to_snake_case(name);
                    result.push_str(&format!(
                        "{indent}let mut {}: Option<{}> = None;\n",
                        snake_name,
                        ty.to_rust_string()
                    ));
                }

                // Use std::panic::catch_unwind to catch panics (like division by zero)
                // and fall back to except_body
                result.push_str(&format!(
                    "{indent}match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {{\n"
                ));
                self.indent += 1;

                // Set hoisted vars for unwrap() in try body (for return statements)
                let _old_try_hoisted_try = std::mem::replace(
                    &mut self.try_hoisted_vars,
                    hoisted_vars
                        .iter()
                        .map(|(name, _)| to_snake_case(name))
                        .collect(),
                );

                // V1.5.2: Set in_try_body flag - closure returns (), so ? is not allowed
                let old_in_try_body = self.in_try_body;
                self.in_try_body = true;

                // Emit try body - convert VarDecl to assignments if hoisting
                for (i, node) in try_body.iter().enumerate() {
                    let is_last = i == try_body.len() - 1;

                    // Handle VarDecl specially if hoisting
                    if need_hoisting {
                        if let IrNode::VarDecl {
                            name,
                            init: Some(expr),
                            ..
                        } = node
                        {
                            // Convert to assignment: var = Some(value);
                            let inner_indent = "    ".repeat(self.indent);
                            let snake_name = to_snake_case(name);
                            result.push_str(&format!(
                                "{}{} = Some({});\n",
                                inner_indent,
                                snake_name,
                                self.emit_expr(expr)
                            ));
                            continue;
                        }
                    }

                    if is_last {
                        // For the last statement, if it's a return, emit just the expression
                        match node {
                            IrNode::Return(Some(expr)) => {
                                let inner_indent = "    ".repeat(self.indent);
                                result.push_str(&format!(
                                    "{}{}\n",
                                    inner_indent,
                                    self.emit_expr(expr)
                                ));
                            }
                            _ => {
                                result.push_str(&self.emit_node(node));
                                result.push('\n');
                            }
                        }
                    } else {
                        result.push_str(&self.emit_node(node));
                        result.push('\n');
                    }
                }

                self.indent -= 1;

                // V1.5.2: Restore in_try_body flag after try body
                self.in_try_body = old_in_try_body;

                result.push_str(&format!("{indent}}})) {{\n"));

                // V1.5.2: Ok case - execute else block if present, otherwise return the value
                if let Some(else_nodes) = else_body {
                    // Set hoisted vars for unwrap() in else body
                    let old_try_hoisted = std::mem::replace(
                        &mut self.try_hoisted_vars,
                        hoisted_vars
                            .iter()
                            .map(|(name, _)| to_snake_case(name))
                            .collect(),
                    );

                    result.push_str(&format!("{indent}    Ok(_) => {{\n"));
                    self.indent += 2;
                    for node in else_nodes {
                        match node {
                            IrNode::Return(Some(expr)) => {
                                let inner_indent = "    ".repeat(self.indent);
                                result.push_str(&format!(
                                    "{}{}\n",
                                    inner_indent,
                                    self.emit_expr(expr)
                                ));
                            }
                            _ => {
                                result.push_str(&self.emit_node(node));
                                result.push('\n');
                            }
                        }
                    }
                    self.indent -= 2;
                    result.push_str(&format!("{indent}    }}\n"));

                    // Restore old try_hoisted_vars
                    self.try_hoisted_vars = old_try_hoisted;
                } else {
                    result.push_str(&format!("{indent}    Ok(__val) => __val,\n"));
                }

                // V1.5.2: Err case - if except_var is defined, bind panic info to it
                if let Some(var_name) = except_var {
                    result.push_str(&format!("{indent}    Err(__exc) => {{\n"));
                    // V1.5.2: Convert panic info appropriately based on may_raise
                    if self.current_func_may_raise {
                        // may_raise function: use TsuchinokoError for raise from compatibility
                        result.push_str(&format!(
                            "{indent}        let {} = TsuchinokoError::new(\"Exception\", &format!(\"{{:?}}\", __exc), None);\n",
                            to_snake_case(var_name)
                        ));
                    } else {
                        // non-may_raise function: use String to avoid TsuchinokoError dependency
                        result.push_str(&format!(
                            "{indent}        let {}: String = if let Some(s) = __exc.downcast_ref::<&str>() {{ s.to_string() }} else if let Some(s) = __exc.downcast_ref::<String>() {{ s.clone() }} else {{ \"Unknown panic\".to_string() }};\n",
                            to_snake_case(var_name)
                        ));
                    }
                } else {
                    result.push_str(&format!("{indent}    Err(_) => {{\n"));
                }

                // Set hoisted vars for unwrap() in except body
                let old_try_hoisted_except = std::mem::replace(
                    &mut self.try_hoisted_vars,
                    hoisted_vars
                        .iter()
                        .map(|(name, _)| to_snake_case(name))
                        .collect(),
                );

                self.indent += 2;
                for node in except_body {
                    // Emit return as proper return statement in except body
                    match node {
                        IrNode::Return(Some(expr)) => {
                            let inner_indent = "    ".repeat(self.indent);
                            result.push_str(&format!(
                                "{}return {};\n",
                                inner_indent,
                                self.emit_expr(expr)
                            ));
                        }
                        _ => {
                            result.push_str(&self.emit_node(node));
                            result.push('\n');
                        }
                    }
                }
                self.indent -= 2;

                // Restore old try_hoisted_vars
                self.try_hoisted_vars = old_try_hoisted_except;

                result.push_str(&format!("{indent}    }}\n"));
                result.push_str(&format!("{indent}}}\n"));

                // V1.5.0: Emit finally block after the match
                if let Some(finally_nodes) = finally_body {
                    result.push_str(&format!("{indent}// finally block\n"));
                    for node in finally_nodes {
                        result.push_str(&self.emit_node(node));
                        result.push('\n');
                    }
                }

                // V1.5.2: Add hoisted vars to try_hoisted_vars for rest of function
                // Variables defined in try need unwrap() even after the try block
                for (name, _) in &hoisted_vars {
                    let snake_name = to_snake_case(name);
                    if !self.try_hoisted_vars.contains(&snake_name) {
                        self.try_hoisted_vars.push(snake_name);
                    }
                }

                result
            }
            IrNode::ImplBlock {
                struct_name,
                methods,
            } => {
                let mut result = format!("{indent}impl {struct_name} {{\n");
                self.indent += 1;
                for method in methods {
                    result.push_str(&self.emit_node(method));
                    result.push('\n');
                }
                self.indent -= 1;
                result.push_str(&format!("{indent}}}\n"));
                result
            }
            IrNode::MethodDecl {
                name,
                params,
                ret,
                body,
                takes_self,
                takes_mut_self,
                may_raise,
            } => {
                let inner_indent = "    ".repeat(self.indent);
                let self_param = if !*takes_self {
                    ""
                } else if *takes_mut_self {
                    "&mut self, "
                } else {
                    "&self, "
                };

                let params_str: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", to_snake_case(n), t.to_rust_string()))
                    .collect();

                // V1.5.2: Use Result type if may_raise
                let ret_str = if *may_raise {
                    self.uses_tsuchinoko_error = true;
                    format!(" -> Result<{}, TsuchinokoError>", ret.to_rust_string())
                } else if *ret == Type::Unit {
                    "".to_string()
                } else {
                    format!(" -> {}", ret.to_rust_string())
                };

                let mut result = format!(
                    "{}fn {}({}{}){} {{\n",
                    inner_indent,
                    to_snake_case(name),
                    self_param,
                    params_str.join(", "),
                    ret_str
                );

                // V1.5.2: Track may_raise for return statement wrapping
                let old_may_raise = self.current_func_may_raise;
                self.current_func_may_raise = *may_raise;

                self.indent += 1;
                for node in body {
                    result.push_str(&self.emit_node(node));
                    result.push('\n');
                }

                // V1.5.2: Add implicit Ok(()) for may_raise methods returning Unit
                if *may_raise && *ret == Type::Unit {
                    let ok_indent = "    ".repeat(self.indent);
                    result.push_str(&format!("{}Ok(())\n", ok_indent));
                }

                self.current_func_may_raise = old_may_raise;
                self.indent -= 1;
                result.push_str(&format!("{inner_indent}}}"));
                result
            }
            IrNode::Raise {
                exc_type,
                message,
                cause,
                line,
            } => {
                let msg_str = self.emit_expr(message);

                // V1.5.2: Inside try body, use panic! so catch_unwind can catch it
                if self.in_try_body {
                    // Inside try block: use panic! for catch_unwind to catch
                    format!("{indent}panic!(\"[{}] {{}}\", {});", exc_type, msg_str)
                } else {
                    // Outside try block: generate Err(TsuchinokoError::...)
                    match cause {
                        Some(cause_expr) => {
                            // With cause: Err(Box::new(TsuchinokoError::with_line("Type", "msg", line, Some(cause))))
                            format!(
                                "{indent}return Err(Box::new(TsuchinokoError::with_line(\"{}\", &format!(\"{{}}\", {}), {}, Some({}))));",
                                exc_type,
                                msg_str,
                                line,
                                self.emit_expr(cause_expr)
                            )
                        }
                        None => {
                            // Without cause: Err(Box::new(TsuchinokoError::with_line("Type", "msg", line, None)))
                            format!(
                                "{indent}return Err(Box::new(TsuchinokoError::with_line(\"{}\", &format!(\"{{}}\", {}), {}, None)));",
                                exc_type,
                                msg_str,
                                line
                            )
                        }
                    }
                }
            }
            IrNode::Break => {
                format!("{indent}break;")
            }
            IrNode::Continue => {
                format!("{indent}continue;")
            }
            // V1.3.0: Assert statement
            IrNode::Assert { test, msg } => {
                let test_str = self.emit_expr(test);
                match msg {
                    Some(m) => format!("{indent}assert!({}, {});", test_str, self.emit_expr(m)),
                    None => format!("{indent}assert!({test_str});"),
                }
            }
            IrNode::Sequence(nodes) => {
                // Emit all nodes in sequence (e.g., StructDef + ImplBlock)
                nodes
                    .iter()
                    .map(|n| self.emit_node_internal(n))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            IrNode::PyO3Import {
                module,
                alias,
                items,
            } => {
                // Mark that PyO3 is needed and track the import
                self.uses_pyo3 = true;
                if let Some(ref item_list) = items {
                    // "from module import a, b, c"
                    for item in item_list {
                        self.external_imports.push((module.clone(), item.clone()));
                    }
                } else {
                    // "import module" or "import module as alias"
                    let effective_alias = alias.clone().unwrap_or_else(|| module.clone());
                    self.external_imports
                        .push((module.clone(), effective_alias));
                }

                // Don't emit anything here - imports are handled in main wrapper
                String::new()
            }
            // V1.6.0: Scoped block (from with statement)
            IrNode::Block { stmts } => {
                let inner: Vec<String> = stmts.iter().map(|s| self.emit_node(s)).collect();
                let inner_code = inner.join("\n");
                format!("{{\n{inner_code}\n}}")
            }
            // V1.6.0: DynamicValue enum definition (for isinstance)
            IrNode::DynamicEnumDef { name, variants } => {
                let mut result = format!("{indent}#[derive(Clone, Debug)]\n");
                result.push_str(&format!("{indent}enum {name} {{\n"));
                for (variant_name, inner_ty) in variants {
                    let rust_type = inner_ty.to_rust_string();
                    result.push_str(&format!("{indent}    {variant_name}({rust_type}),\n"));
                }
                result.push_str(&format!("{indent}}}"));
                result
            }
            // V1.6.0: match expression (for isinstance)
            IrNode::Match { value, arms } => {
                let value_str = self.emit_expr(value);
                let mut result = format!("{indent}match {value_str} {{\n");
                for arm in arms {
                    let variant = &arm.variant;
                    let binding = &arm.binding;
                    result.push_str(&format!(
                        "{indent}    DynamicValue::{variant}({binding}) => {{\n"
                    ));
                    self.indent += 2;
                    for stmt in &arm.body {
                        result.push_str(&self.emit_node(stmt));
                        result.push('\n');
                    }
                    self.indent -= 2;
                    result.push_str(&format!("{indent}    }}\n"));
                }
                result.push_str(&format!("{indent}}}"));
                result
            }
        }
    }

    fn emit_expr_internal(&mut self, expr: &IrExpr) -> String {
        match expr {
            IrExpr::IntLit(n) => format!("{n}i64"),
            IrExpr::FloatLit(f) => format!("{f:.1}"),
            IrExpr::StringLit(s) => format!("\"{s}\""),
            IrExpr::BoolLit(b) => b.to_string(),
            IrExpr::NoneLit => "None".to_string(),
            IrExpr::Var(name) => {
                // Don't snake_case qualified paths like std::collections::HashMap
                let var_name = if name.contains("::") {
                    name.clone()
                } else if name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    // Don't snake_case class names (PascalCase)
                    name.clone()
                } else {
                    to_snake_case(name)
                };

                // V1.5.2: Skip unwrap if variable is shadowed (e.g., for loop variable)
                let is_shadowed = self.shadowed_vars.contains(&var_name);

                // Check if this variable needs unwrap() due to hoisting
                // 1. try_hoisted_vars: variables from try block (need clone due to closure)
                // 2. current_hoisted_vars: variables hoisted at function level (if/for etc.)
                let is_try_hoisted = !is_shadowed && self.try_hoisted_vars.contains(&var_name);
                let is_func_hoisted = !is_shadowed
                    && self
                        .current_hoisted_vars
                        .iter()
                        .any(|v| to_snake_case(&v.name) == var_name);

                if is_try_hoisted {
                    // Try block hoisting needs clone due to catch_unwind closure
                    format!("{}.clone().unwrap()", var_name)
                } else if is_func_hoisted {
                    // Function-level hoisting (if/for blocks) - no clone needed
                    format!("{}.unwrap()", var_name)
                } else {
                    var_name
                }
            }
            IrExpr::BinOp { left, op, right } => {
                if let IrBinOp::Pow = op {
                    return format!(
                        "({} as i64).pow(({}) as u32)",
                        self.emit_expr(left),
                        self.emit_expr(right)
                    );
                }

                let op_str = match op {
                    IrBinOp::Add => "+",
                    IrBinOp::Sub => "-",
                    IrBinOp::Mul => "*",
                    IrBinOp::Div => "/",
                    IrBinOp::Mod => "%",
                    IrBinOp::Eq => "==",
                    IrBinOp::NotEq => "!=",
                    IrBinOp::Lt => "<",
                    IrBinOp::Gt => ">",
                    IrBinOp::LtEq => "<=",
                    IrBinOp::GtEq => ">=",
                    IrBinOp::And => "&&",
                    IrBinOp::Or => "||",
                    IrBinOp::FloorDiv => "/",
                    IrBinOp::Pow => unreachable!(),
                    // Bitwise operators (V1.3.0)
                    IrBinOp::BitAnd => "&",
                    IrBinOp::BitOr => "|",
                    IrBinOp::BitXor => "^",
                    IrBinOp::Shl => "<<",
                    IrBinOp::Shr => ">>",
                    IrBinOp::MatMul => {
                        // V1.3.0: a @ b -> py_bridge.call_json("numpy.matmul", &[a, b])
                        // For NumPy arrays, use the resident worker
                        // V1.5.2: Use ? instead of unwrap() for error propagation
                        self.current_func_may_raise = true;
                        self.uses_tsuchinoko_error = true;
                        return format!(
                            "py_bridge.call_json::<tsuchinoko::bridge::protocol::TnkValue>(\"numpy.matmul\", &[serde_json::json!({}), serde_json::json!({})]).map_err(|e| TsuchinokoError::new(\"ExternalError\", &e, None))?",
                            self.emit_expr(left),
                            self.emit_expr(right)
                        );
                    }
                    IrBinOp::Contains => {
                        // x in collection -> collection.contains(&x)
                        // For dicts, use contains_key - handle via ContainsKey variant
                        return format!(
                            "{}.contains(&{})",
                            self.emit_expr(right),
                            self.emit_expr(left)
                        );
                    }
                    IrBinOp::NotContains => {
                        // x not in collection -> !collection.contains(&x) (V1.3.0)
                        return format!(
                            "!{}.contains(&{})",
                            self.emit_expr(right),
                            self.emit_expr(left)
                        );
                    }
                    IrBinOp::Is => {
                        // x is None -> x.is_none()
                        let right_str = self.emit_expr(right);
                        if right_str == "None" {
                            return format!("{}.is_none()", self.emit_expr(left));
                        } else {
                            // General case: std::ptr::eq or ==
                            return format!("({} == {})", self.emit_expr(left), right_str);
                        }
                    }
                    IrBinOp::IsNot => {
                        // x is not None -> x.is_some()
                        let right_str = self.emit_expr(right);
                        if right_str == "None" {
                            return format!("{}.is_some()", self.emit_expr(left));
                        } else {
                            // General case: !=
                            return format!("({} != {})", self.emit_expr(left), right_str);
                        }
                    }
                };
                format!(
                    "({} {} {})",
                    self.emit_expr(left),
                    op_str,
                    self.emit_expr(right)
                )
            }
            IrExpr::UnaryOp { op, operand } => {
                let op_str = match op {
                    IrUnaryOp::Neg => "-",
                    IrUnaryOp::Not => "!",
                    IrUnaryOp::Deref => "*",
                    IrUnaryOp::BitNot => "!", // V1.3.0 - Rust uses ! for bitwise NOT too
                };
                format!("({}{})", op_str, self.emit_expr(operand))
            }
            IrExpr::Call {
                func,
                args,
                callee_may_raise,
            } => {
                let is_print = if let IrExpr::Var(name) = func.as_ref() {
                    name == "print"
                } else {
                    false
                };

                if is_print {
                    // Handle print("msg", arg) -> println!("msg {:?}", arg)
                    // Clean up: remove .to_string() for string literals and .clone() for println
                    let cleaned_args: Vec<_> = args
                        .iter()
                        .map(|a| {
                            // Unwrap unnecessary MethodCall wrappers
                            let unwrapped = match a {
                                IrExpr::MethodCall {
                                    target,
                                    method,
                                    args: mc_args,
                                    target_type: _,
                                } if mc_args.is_empty()
                                    && (method == "clone" || method == "to_string") =>
                                {
                                    target.as_ref()
                                }
                                other => other,
                            };

                            // For string literals, emit directly
                            match unwrapped {
                                IrExpr::StringLit(s) => format!("\"{s}\""),
                                _ => {
                                    // Just pass by ref for println
                                    let expr_str = self.emit_expr(unwrapped);
                                    if expr_str.starts_with('&') {
                                        expr_str
                                    } else {
                                        format!("&{expr_str}")
                                    }
                                }
                            }
                        })
                        .collect();

                    let format_string = std::iter::repeat_n("{:?}", args.len())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if args.is_empty() {
                        "println!()".to_string()
                    } else {
                        format!(
                            "println!(\"{}\", {})",
                            format_string,
                            cleaned_args.join(", ")
                        )
                    }
                } else {
                    // Check if variable (possible struct constructor or function name)
                    let func_name_opt = if let IrExpr::Var(name) = func.as_ref() {
                        Some(name.clone())
                    } else {
                        None
                    };

                    if let Some(name) = func_name_opt {
                        // V1.4.0: Check if this is a from-imported function
                        // external_imports contains (module, item) tuples
                        // If name matches any item, convert to py_bridge.call_json("module.item", ...)
                        let from_import_module = self
                            .external_imports
                            .iter()
                            .find(|(_, item)| item == &name)
                            .map(|(module, _)| module.clone());

                        if let Some(module) = from_import_module {
                            // This is a from-imported function - use py_bridge
                            self.needs_resident = true;
                            let args_str: Vec<_> = args
                                .iter()
                                .map(|a| {
                                    format!(
                                        "serde_json::json!({})",
                                        self.emit_expr_no_outer_parens(a)
                                    )
                                })
                                .collect();
                            return format!(
                                "py_bridge.call_json::<tsuchinoko::bridge::protocol::TnkValue>(\"{}.{}\", &[{}]).unwrap().as_f64().unwrap()",
                                module,
                                name,
                                args_str.join(", ")
                            );
                        }

                        // V1.3.1: int/float/str are now handled by semantic analyzer
                        // and converted to IrExpr::Cast or IrExpr::MethodCall
                        // V1.3.1: Struct constructors are now handled by semantic analyzer
                        // and converted to IrExpr::StructConstruct

                        {
                            let mut args_str: Vec<_> = args
                                .iter()
                                .map(|a| self.emit_expr_no_outer_parens(a))
                                .collect();
                            // Don't snake_case built-in Rust expressions or qualified paths
                            let func_name = if name == "Some"
                                || name == "None"
                                || name == "Ok"
                                || name == "Err"
                                || name.contains("::")
                            {
                                name.clone()
                            } else if name == "main" {
                                "main_py".to_string()
                            } else {
                                to_snake_case(&name)
                            };
                            // resident_functions に登録された関数なら py_bridge を追加
                            if self.resident_functions.contains(&func_name) {
                                self.needs_resident = true;
                                if self.indent > 0 && name != "main" && name != "__top_level__" {
                                    // Make sure we check if we are actually INSIDE a function that has py_bridge argument.
                                    // self.indent > 0 implies inside a block/function.
                                    // A more robust way is to track "current_function_has_bridge".
                                    // For now, assuming indent > 0 and not main means we are inside a resident function (because of propagation).
                                    // BUT, we need to be carefully distinguishes between "inside main" and "inside other resident func".
                                    // We'll rely on a new field `is_inside_resident_func`.
                                    if self.is_inside_resident_func {
                                        args_str.insert(0, "py_bridge".to_string());
                                    } else {
                                        args_str.insert(0, "&mut py_bridge".to_string());
                                    }
                                } else {
                                    args_str.insert(0, "&mut py_bridge".to_string());
                                }
                            }
                            let call_str = format!("{}({})", func_name, args_str.join(", "));

                            // V1.5.2: If calling a may_raise function from a non-may_raise context, add .unwrap()
                            // Use IR's callee_may_raise instead of tracking in emitter
                            // Also, in try body closure, use .unwrap() instead of ? (closure returns ())
                            if *callee_may_raise
                                && (!self.current_func_may_raise || self.in_try_body)
                            {
                                format!("{}.unwrap()", call_str)
                            } else if *callee_may_raise
                                && self.current_func_may_raise
                                && !self.in_try_body
                            {
                                // Both caller and callee may raise, and not in try body - use ?
                                format!("{}?", call_str)
                            } else {
                                call_str
                            }
                        }
                    } else {
                        // Generic function call (func is expression)
                        let func_str = self.emit_expr(func);
                        let args_str: Vec<_> = args
                            .iter()
                            .map(|a| self.emit_expr_no_outer_parens(a))
                            .collect();
                        // If func is a FieldAccess, we need (target.field)(args) syntax in Rust
                        // to call a function stored in a field
                        let needs_parens = matches!(func.as_ref(), IrExpr::FieldAccess { .. });
                        if needs_parens {
                            format!("({})({})", func_str, args_str.join(", "))
                        } else {
                            format!("{}({})", func_str, args_str.join(", "))
                        }
                    }
                }
            }
            IrExpr::List {
                elem_type,
                elements,
            } => {
                let elems: Vec<_> = elements
                    .iter()
                    .map(|e| {
                        let mut s = self.emit_expr(e);
                        // If element type is String and value is a string literal, add .to_string()
                        if matches!(elem_type, Type::String)
                            && s.starts_with('"')
                            && !s.contains(".to_string()")
                        {
                            s = format!("{s}.to_string()");
                        }
                        // If element type is Tuple with String, convert string literals inside
                        if let Type::Tuple(inner_types) = elem_type {
                            if inner_types.iter().any(|t| matches!(t, Type::String)) {
                                // Replace string literals inside tuple with .to_string() version
                                // e.g., ("a", 1i64) -> ("a".to_string(), 1i64)
                                if s.starts_with('(') && s.ends_with(')') {
                                    let inner = &s[1..s.len() - 1];
                                    let parts: Vec<&str> = inner.split(", ").collect();
                                    let converted: Vec<String> = parts
                                        .iter()
                                        .enumerate()
                                        .map(|(i, part)| {
                                            if i < inner_types.len()
                                                && matches!(inner_types[i], Type::String)
                                                && part.starts_with('"')
                                                && !part.contains(".to_string()")
                                            {
                                                format!("{part}.to_string()")
                                            } else {
                                                part.to_string()
                                            }
                                        })
                                        .collect();
                                    s = format!("({})", converted.join(", "));
                                }
                            }
                        }
                        s
                    })
                    .collect();
                format!("vec![{}]", elems.join(", "))
            }
            IrExpr::Dict {
                key_type,
                value_type,
                entries,
            } => {
                let use_json = matches!(key_type, Type::Any) || matches!(value_type, Type::Any);

                if entries.is_empty() {
                    if use_json {
                        "serde_json::json!({})".to_string()
                    } else {
                        "std::collections::HashMap::new()".to_string()
                    }
                } else {
                    let pairs: Vec<_> = entries
                        .iter()
                        .map(|(k, v)| {
                            let mut key_str = self.emit_expr_no_outer_parens(k);
                            let mut val_str = self.emit_expr_no_outer_parens(v);

                            if !use_json {
                                // For HashMap, we need owned Strings if the type is String
                                if matches!(key_type, Type::String)
                                    && key_str.starts_with('"')
                                    && !key_str.contains(".to_string()")
                                {
                                    key_str = format!("{key_str}.to_string()");
                                }
                                if matches!(value_type, Type::String)
                                    && val_str.starts_with('"')
                                    && !val_str.contains(".to_string()")
                                {
                                    val_str = format!("{val_str}.to_string()");
                                }
                                format!("({key_str}, {val_str})")
                            } else {
                                format!("{key_str}: {val_str}")
                            }
                        })
                        .collect();

                    if use_json {
                        format!("serde_json::json!({{ {} }})", pairs.join(", "))
                    } else {
                        format!("std::collections::HashMap::from([{}])", pairs.join(", "))
                    }
                }
            }
            // V1.5.0: Set literal
            IrExpr::Set {
                elem_type,
                elements,
            } => {
                if elements.is_empty() {
                    "std::collections::HashSet::new()".to_string()
                } else {
                    let elems: Vec<_> = elements
                        .iter()
                        .map(|e| {
                            let mut s = self.emit_expr_no_outer_parens(e);
                            // For String type, add .to_string() to literals
                            if matches!(elem_type, Type::String)
                                && s.starts_with('"')
                                && !s.contains(".to_string()")
                            {
                                s = format!("{s}.to_string()");
                            }
                            s
                        })
                        .collect();
                    format!("std::collections::HashSet::from([{}])", elems.join(", "))
                }
            }
            IrExpr::FString { parts, values } => {
                // Generate format string: "{:?}{:?}{:?}" from parts
                // Use {:?} (Debug) instead of {} (Display) to support Vec and other types
                let format_str: String = parts
                    .iter()
                    .enumerate()
                    .map(|(i, part)| {
                        if i < parts.len() - 1 {
                            format!("{part}{{:?}}")
                        } else {
                            part.clone()
                        }
                    })
                    .collect();

                let value_strs: Vec<_> =
                    values.iter().map(|v| self.emit_expr_internal(v)).collect();

                if values.is_empty() {
                    format!("\"{}\"", parts.join(""))
                } else {
                    format!("format!(\"{}\", {})", format_str, value_strs.join(", "))
                }
            }
            IrExpr::IfExp { test, body, orelse } => {
                format!(
                    "if {} {{ {} }} else {{ {} }}",
                    self.emit_expr_internal(test),
                    self.emit_expr_internal(body),
                    self.emit_expr_internal(orelse)
                )
            }
            IrExpr::ListComp {
                elt,
                target,
                iter,
                condition,
            } => {
                // Use .iter().cloned() to avoid ownership transfer
                // This allows the same collection to be used multiple times
                let elt_str = self.emit_expr_internal(elt);

                let target_has_comma = target.contains(',');
                let target_snake = if target_has_comma {
                    let parts: Vec<String> =
                        target.split(',').map(|s| to_snake_case(s.trim())).collect();
                    format!("({})", parts.join(", "))
                } else {
                    to_snake_case(target)
                };

                // For tuple unpacking, always use the target name to avoid partial usage check complexity
                let closure_var = if target_has_comma || elt_str.contains(&target_snake) {
                    target_snake.clone()
                } else {
                    "_".to_string()
                };

                let iter_str = self.emit_expr_internal(iter);

                let iter_chain = match iter.as_ref() {
                    // Range needs parentheses for method chaining: (1..10).filter(...)
                    IrExpr::Range { .. } => format!("({iter_str})"),
                    // MethodCall to items() returns a Vec - use .into_iter() for ownership
                    IrExpr::MethodCall { method, .. } if method == "items" => {
                        format!("{iter_str}.into_iter()")
                    }
                    // Already an iterator (MethodCall with iter/filter/map), use directly
                    IrExpr::MethodCall { method, .. }
                        if method.contains("iter")
                            || method.contains("filter")
                            || method.contains("map") =>
                    {
                        iter_str
                    }
                    // Collection: use .iter().cloned() to borrow and copy values
                    _ => format!("{iter_str}.iter().cloned()"),
                };

                if let Some(cond) = condition {
                    let cond_str = self.emit_expr_internal(cond);
                    // Use pattern without & for filter - references are handled by the condition
                    format!(
                        "{}.filter(|{}| {}).map(|{}| {}).collect::<Vec<_>>()",
                        iter_chain, &target_snake, cond_str, closure_var, elt_str
                    )
                } else {
                    format!("{iter_chain}.map(|{closure_var}| {elt_str}).collect::<Vec<_>>()")
                }
            }
            // V1.3.0: Dict comprehension {k: v for target in iter if condition}
            IrExpr::DictComp {
                key,
                value,
                target,
                iter,
                condition,
            } => {
                let key_str = self.emit_expr_internal(key);
                let value_str = self.emit_expr_internal(value);

                let target_has_comma = target.contains(',');
                let target_snake = if target_has_comma {
                    let parts: Vec<String> =
                        target.split(',').map(|s| to_snake_case(s.trim())).collect();
                    format!("({})", parts.join(", "))
                } else {
                    to_snake_case(target)
                };

                let iter_str = self.emit_expr_internal(iter);

                let iter_chain = match iter.as_ref() {
                    IrExpr::Range { .. } => format!("({iter_str})"),
                    IrExpr::MethodCall { method, .. } if method == "items" => {
                        format!("{iter_str}.into_iter()")
                    }
                    IrExpr::MethodCall { method, .. }
                        if method.contains("iter")
                            || method.contains("filter")
                            || method.contains("map") =>
                    {
                        iter_str
                    }
                    _ => format!("{iter_str}.iter().cloned()"),
                };

                if let Some(cond) = condition {
                    let cond_str = self.emit_expr_internal(cond);
                    format!(
                        "{}.filter(|&{}| {}).map(|{}| ({}, {})).collect::<std::collections::HashMap<_, _>>()",
                        iter_chain, &target_snake, cond_str, &target_snake, key_str, value_str
                    )
                } else {
                    format!(
                        "{}.map(|{}| ({}, {})).collect::<std::collections::HashMap<_, _>>()",
                        iter_chain, &target_snake, key_str, value_str
                    )
                }
            }
            // V1.6.0: Set comprehension {x for target in iter if condition}
            IrExpr::SetComp {
                elt,
                target,
                iter,
                condition,
            } => {
                let elt_str = self.emit_expr_internal(elt);

                let target_has_comma = target.contains(',');
                let target_snake = if target_has_comma {
                    let parts: Vec<String> =
                        target.split(',').map(|s| to_snake_case(s.trim())).collect();
                    format!("({})", parts.join(", "))
                } else {
                    to_snake_case(target)
                };

                let closure_var = if target_has_comma || elt_str.contains(&target_snake) {
                    target_snake.clone()
                } else {
                    "_".to_string()
                };

                let iter_str = self.emit_expr_internal(iter);

                let iter_chain = match iter.as_ref() {
                    IrExpr::Range { .. } => format!("({iter_str})"),
                    IrExpr::MethodCall { method, .. }
                        if method.contains("iter")
                            || method.contains("filter")
                            || method.contains("map") =>
                    {
                        iter_str
                    }
                    _ => format!("{iter_str}.iter().cloned()"),
                };

                if let Some(cond) = condition {
                    let cond_str = self.emit_expr_internal(cond);
                    format!(
                        "{}.filter(|{}| {}).map(|{}| {}).collect::<std::collections::HashSet<_>>()",
                        iter_chain, &target_snake, cond_str, closure_var, elt_str
                    )
                } else {
                    format!(
                        "{iter_chain}.map(|{closure_var}| {elt_str}).collect::<std::collections::HashSet<_>>()"
                    )
                }
            }
            IrExpr::Closure {
                params,
                body,
                ret_type,
            } => {
                let params_str: Vec<String> = params.iter().map(|p| to_snake_case(p)).collect();

                // Increase indent for closure body is tricky because emit_expr_internal doesn't mutate state?
                // But emit_node uses self.indent_level.
                // Assuming we can't mutate self here easily if reference is shared?
                // Wait, emit_expr takes &self.
                // If indent_level is in RefCell checking struct def will tell.
                // If not, we might produce ugly indentation or need refactoring.
                // For now, let's assume we just emit body directly and let rustfmt handle it,
                // OR clean code manually processing lines?
                // "    " + line.

                let mut body_str = String::new();
                for (i, stmt) in body.iter().enumerate() {
                    let is_last = i == body.len() - 1;
                    let stmt_str = if is_last {
                        match stmt {
                            IrNode::Expr(e) => {
                                format!("{}{}", "    ".repeat(self.indent + 1), self.emit_expr(e))
                            }
                            _ => self.emit_node(stmt),
                        }
                    } else {
                        self.emit_node(stmt)
                    };
                    for line in stmt_str.lines() {
                        body_str.push_str("    ");
                        body_str.push_str(line);
                        body_str.push('\n');
                    }
                }

                let ret_str = if let Type::Unit = ret_type {
                    "".to_string()
                } else if let Type::Unknown = ret_type {
                    "".to_string()
                } else {
                    format!(" -> {}", ret_type.to_rust_string())
                };

                format!(
                    "move |{}|{} {{\n{}\n}}",
                    params_str.join(", "),
                    ret_str,
                    body_str
                )
            }
            IrExpr::BoxNew(arg) => {
                // Use Arc::new for Callable fields (which are Arc<dyn Fn>)
                format!("std::sync::Arc::new({})", self.emit_expr(arg))
            }
            IrExpr::Cast { target, ty } => {
                format!("({} as {})", self.emit_expr(target), ty)
            }
            IrExpr::RawCode(code) => code.clone(),
            IrExpr::JsonConversion { target, convert_to } => {
                let target_code = self.emit_expr_internal(target);
                match convert_to.as_str() {
                    "f64" => format!("{target_code}.as_f64().unwrap()"),
                    "i64" => format!("{target_code}.as_i64().unwrap()"),
                    "String" => format!("{target_code}.as_str().unwrap().to_string()"),
                    "bool" => format!("{target_code}.as_bool().unwrap()"),
                    _ => target_code,
                }
            }
            IrExpr::Tuple(elements) => {
                let elems: Vec<_> = elements.iter().map(|e| self.emit_expr(e)).collect();
                format!("({})", elems.join(", "))
            }
            IrExpr::Index { target, index } => {
                // Handle negative index: arr[-1] -> arr[arr.len()-1]
                // Helper function to extract negative index value
                fn extract_negative_index(expr: &IrExpr) -> Option<i64> {
                    match expr {
                        // Case 1: UnaryOp { Neg, IntLit(n) }
                        IrExpr::UnaryOp {
                            op: IrUnaryOp::Neg,
                            operand,
                        } => {
                            if let IrExpr::IntLit(n) = operand.as_ref() {
                                return Some(*n);
                            }
                        }
                        // Case 2: IntLit with negative value
                        IrExpr::IntLit(n) if *n < 0 => {
                            return Some(n.abs());
                        }
                        // Case 3: Cast { target: ..., ty } - unwrap and recurse
                        IrExpr::Cast { target, .. } => {
                            return extract_negative_index(target);
                        }
                        _ => {}
                    }
                    None
                }

                if let Some(abs_val) = extract_negative_index(index) {
                    let target_str = self.emit_expr(target);
                    return format!("{target_str}[{target_str}.len() - {abs_val}]");
                }
                format!("{}[{}]", self.emit_expr(target), self.emit_expr(index))
            }
            IrExpr::Slice {
                target,
                start,
                end,
                step,
            } => {
                // Handle Python-style slices: [:n], [n:], [s:e], [:], [::2], [::-1], [1:8:2]
                // Python slices never panic on out-of-bounds, they clamp to valid range
                let target_str = self.emit_expr(target);

                // V1.5.0: Handle step slices
                if let Some(step_expr) = step {
                    let step_val = self.emit_expr(step_expr);

                    // Check if step is -1 (reverse) - could be IntLit(-1) or UnaryOp(Neg, IntLit(1))
                    let is_reverse = matches!(step_expr.as_ref(), IrExpr::IntLit(-1))
                        || matches!(step_expr.as_ref(),
                            IrExpr::UnaryOp { op: IrUnaryOp::Neg, operand }
                            if matches!(operand.as_ref(), IrExpr::IntLit(1)));

                    if is_reverse {
                        // [::-1] -> .iter().rev().cloned().collect()
                        return match (start, end) {
                            (None, None) => {
                                format!("{target_str}.iter().rev().cloned().collect::<Vec<_>>()")
                            }
                            (Some(s), None) => {
                                // [n::-1] is Python's reverse from index n down to 0
                                format!("({{ let v = &{target_str}; let s = ({}).min(v.len() as i64 - 1).max(0) as usize; v[..=s].iter().rev().cloned().collect::<Vec<_>>() }})", self.emit_expr(s))
                            }
                            (None, Some(e)) => {
                                // [:n:-1] - reverse ending at index n (exclusive)
                                format!("({{ let v = &{target_str}; let e = ({}).max(0) as usize; v[e..].iter().rev().cloned().collect::<Vec<_>>() }})", self.emit_expr(e))
                            }
                            (Some(s), Some(e)) => {
                                format!("({{ let v = &{target_str}; let s = ({}).max(0) as usize; let e = ({}).min(v.len() as i64).max(0) as usize; v[s.min(e)..e].iter().rev().cloned().collect::<Vec<_>>() }})", self.emit_expr(s), self.emit_expr(e))
                            }
                        };
                    }

                    // Positive step: [::n] -> .iter().step_by(n).cloned().collect()
                    return match (start, end) {
                        (None, None) => format!("{target_str}.iter().step_by({step_val} as usize).cloned().collect::<Vec<_>>()"),
                        (Some(s), None) => format!("({{ let v = &{target_str}; let s = ({}).max(0) as usize; v[s..].iter().step_by({step_val} as usize).cloned().collect::<Vec<_>>() }})", self.emit_expr(s)),
                        (None, Some(e)) => format!("({{ let v = &{target_str}; let e = ({}).min(v.len() as i64).max(0) as usize; v[..e].iter().step_by({step_val} as usize).cloned().collect::<Vec<_>>() }})", self.emit_expr(e)),
                        (Some(s), Some(e)) => format!("({{ let v = &{target_str}; let s = ({}).max(0) as usize; let e = ({}).min(v.len() as i64).max(0) as usize; v[s.min(e)..e].iter().step_by({step_val} as usize).cloned().collect::<Vec<_>>() }})", self.emit_expr(s), self.emit_expr(e)),
                    };
                }

                // Original slice handling (no step)
                match (start, end) {
                    (None, Some(e)) => {
                        // [:n] -> [..(n as usize).min(len)].to_vec()
                        // Handle negative indices: [:-n] -> [..len().saturating_sub(n)].to_vec()
                        if let IrExpr::UnaryOp {
                            op: IrUnaryOp::Neg,
                            operand,
                        } = e.as_ref()
                        {
                            if let IrExpr::IntLit(n) = operand.as_ref() {
                                return format!(
                                    "({target_str}[..{target_str}.len().saturating_sub({n})].to_vec())"
                                );
                            }
                        }
                        // Clamp end to len to avoid panic: [..min(n, len)]
                        format!(
                            "({}[..({} as usize).min({}.len())].to_vec())",
                            target_str,
                            self.emit_expr(e),
                            target_str
                        )
                    }
                    (Some(s), None) => {
                        // [n:] -> [min(n, len)..].to_vec()
                        // Handle negative indices: [-n:] -> [len().saturating_sub(n)..].to_vec()
                        if let IrExpr::UnaryOp {
                            op: IrUnaryOp::Neg,
                            operand,
                        } = s.as_ref()
                        {
                            if let IrExpr::IntLit(n) = operand.as_ref() {
                                return format!(
                                    "({target_str}[{target_str}.len().saturating_sub({n})..].to_vec())"
                                );
                            }
                        }
                        // Clamp start to len to avoid panic: [min(n, len)..]
                        format!(
                            "({}[({} as usize).min({}.len())..].to_vec())",
                            target_str,
                            self.emit_expr(s),
                            target_str
                        )
                    }
                    (Some(s), Some(e)) => {
                        // [s:e] -> [min(s, len)..min(e, len)].to_vec()
                        // Also ensure start <= end by using start.min(end)
                        format!("({{ let _s = ({} as usize).min({}.len()); let _e = ({} as usize).min({}.len()); {}[_s.min(_e).._e].to_vec() }})", 
                                self.emit_expr(s), target_str, self.emit_expr(e), target_str, target_str)
                    }
                    (None, None) => {
                        // [:] -> clone()
                        format!("{target_str}.clone()")
                    }
                }
            }
            IrExpr::Range { start, end } => {
                format!("{}..{}", self.emit_expr(start), self.emit_expr(end))
            }
            IrExpr::MethodCall {
                target,
                method,
                args,
                target_type,
            } => {
                if args.is_empty() {
                    if method == "len" {
                        format!("{}.{}() as i64", self.emit_expr_internal(target), method)
                    } else if method == "copy" {
                        // Python list.copy() -> Rust .to_vec()
                        format!("{}.to_vec()", self.emit_expr_internal(target))
                    } else if method == "collect_hashset" {
                        // V1.5.0: set() constructor -> .collect::<HashSet<_>>()
                        format!(
                            "{}.collect::<std::collections::HashSet<_>>()",
                            self.emit_expr_internal(target)
                        )
                    } else if method == "pop" {
                        // V1.5.0: Python list.pop() -> Rust list.pop().unwrap()
                        format!("{}.pop().unwrap()", self.emit_expr_internal(target))
                    } else if method == "clear" {
                        // V1.5.0: Python list.clear() -> Rust list.clear()
                        format!("{}.clear()", self.emit_expr_internal(target))
                    // V1.5.0: String is* methods (no args)
                    } else if method == "isdigit" {
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_ascii_digit())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "isalpha" {
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_alphabetic())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "isalnum" {
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_alphanumeric())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "isupper" {
                        format!(
                            "{}.chars().any(|c| c.is_alphabetic()) && {}.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "islower" {
                        format!(
                            "{}.chars().any(|c| c.is_alphabetic()) && {}.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_lowercase())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else {
                        format!("{}.{}()", self.emit_expr_internal(target), method)
                    }
                } else {
                    // V1.5.0: Set method translations
                    // V1.5.2: Check target_type to correctly handle add method
                    if method == "add" && matches!(target_type, Type::Set(_)) {
                        // Python set.add(x) -> Rust set.insert(x)
                        let args_str: Vec<_> =
                            args.iter().map(|a| self.emit_expr_internal(a)).collect();
                        format!(
                            "{}.insert({})",
                            self.emit_expr_internal(target),
                            args_str.join(", ")
                        )
                    } else if method == "discard" {
                        // Python set.discard(x) -> Rust set.remove(&x)
                        // Note: set.remove is also &x, but list.remove is handled differently
                        // in semantic analysis (via try_handle_special_method)
                        let arg = &args[0];
                        format!(
                            "{}.remove(&{})",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(arg)
                        )
                    } else if method == "pop" && args.len() == 1 {
                        // Python list.pop(i) -> Rust list.remove(i as usize)
                        let idx = &args[0];
                        format!(
                            "{}.remove({} as usize)",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(idx)
                        )
                    // Note: list.insert is handled in semantic analysis to distinguish from dict.insert
                    } else if method == "extend" {
                        // Python list.extend(iter) -> Rust list.extend(iter)
                        let iter = &args[0];
                        format!(
                            "{}.extend({})",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(iter)
                        )
                    // V1.5.0: String method translations
                    } else if method == "startswith" {
                        // Python s.startswith("x") -> Rust s.starts_with("x")
                        let arg = &args[0];
                        format!(
                            "{}.starts_with(&{})",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(arg)
                        )
                    } else if method == "endswith" {
                        // Python s.endswith("x") -> Rust s.ends_with("x")
                        let arg = &args[0];
                        format!(
                            "{}.ends_with(&{})",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(arg)
                        )
                    } else if method == "replace" && args.len() >= 2 {
                        // Python s.replace(old, new) -> Rust s.replace(&old, &new)
                        let old = &args[0];
                        let new = &args[1];
                        format!(
                            "{}.replace(&{}, &{})",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(old),
                            self.emit_expr_internal(new)
                        )
                    } else if method == "find" && args.len() == 1 {
                        // Python s.find(sub) -> Rust s.find(&sub).map(|i| i as i64).unwrap_or(-1)
                        let sub = &args[0];
                        format!(
                            "{}.find(&{}).map(|i| i as i64).unwrap_or(-1i64)",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(sub)
                        )
                    } else if method == "rfind" && args.len() == 1 {
                        // Python s.rfind(sub) -> Rust s.rfind(&sub).map(|i| i as i64).unwrap_or(-1)
                        let sub = &args[0];
                        format!(
                            "{}.rfind(&{}).map(|i| i as i64).unwrap_or(-1i64)",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(sub)
                        )
                    } else if method == "isdigit" {
                        // Python s.isdigit() -> Rust s.chars().all(|c| c.is_ascii_digit())
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_ascii_digit())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "isalpha" {
                        // Python s.isalpha() -> Rust s.chars().all(|c| c.is_alphabetic())
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_alphabetic())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "isalnum" {
                        // Python s.isalnum() -> Rust s.chars().all(|c| c.is_alphanumeric())
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_alphanumeric())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "isupper" {
                        // Python s.isupper() -> Rust s.chars().any(|c| c.is_alphabetic()) && s.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())
                        format!(
                            "{}.chars().any(|c| c.is_alphabetic()) && {}.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "islower" {
                        // Python s.islower() -> Rust similar logic
                        format!(
                            "{}.chars().any(|c| c.is_alphabetic()) && {}.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_lowercase())",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(target)
                        )
                    } else if method == "count" && args.len() == 1 {
                        // Python s.count(sub) -> Rust s.matches(&sub).count() as i64
                        let sub = &args[0];
                        format!(
                            "{}.matches(&{}).count() as i64",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(sub)
                        )
                    } else if method == "zfill" && args.len() == 1 {
                        // Python s.zfill(width) -> format!("{:0>width$}", s, width=width)
                        let width = &args[0];
                        format!(
                            "format!(\"{{:0>width$}}\", {}, width = {} as usize)",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(width)
                        )
                    } else if method == "ljust" && !args.is_empty() {
                        // Python s.ljust(width) -> format!("{:<width$}", s)
                        let width = &args[0];
                        format!(
                            "format!(\"{{:<width$}}\", {}, width = {} as usize)",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(width)
                        )
                    } else if method == "rjust" && !args.is_empty() {
                        // Python s.rjust(width) -> format!("{:>width$}", s)
                        let width = &args[0];
                        format!(
                            "format!(\"{{:>width$}}\", {}, width = {} as usize)",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(width)
                        )
                    } else if method == "center" && !args.is_empty() {
                        // Python s.center(width) -> format!("{:^width$}", s)
                        let width = &args[0];
                        format!(
                            "format!(\"{{:^width$}}\", {}, width = {} as usize)",
                            self.emit_expr_internal(target),
                            self.emit_expr_internal(width)
                        )
                    } else {
                        let args_str: Vec<_> =
                            args.iter().map(|a| self.emit_expr_internal(a)).collect();
                        format!(
                            "{}.{}({})",
                            self.emit_expr_internal(target),
                            method,
                            args_str.join(", ")
                        )
                    }
                }
            }
            IrExpr::PyO3MethodCall {
                target,
                method,
                args,
            } => {
                self.needs_resident = true;
                let mut arg_evals = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    arg_evals.push(format!(
                        "let _arg_{} = {};",
                        i,
                        self.emit_expr_internal(arg)
                    ));
                }

                let args_json: Vec<String> = (0..args.len())
                    .map(|i| format!("serde_json::json!(_arg_{i})"))
                    .collect();

                // V1.5.2: Use ? instead of unwrap() for error propagation
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;
                format!(
                    "{{\n{}    py_bridge.call_json_method::<tsuchinoko::bridge::protocol::TnkValue>({}.clone(), {:?}, &[{}]).map_err(|e| TsuchinokoError::new(\"ExternalError\", &e, None))?\n}}",
                    arg_evals.join("\n    ") + "\n",
                    self.emit_expr_internal(target),
                    method,
                    args_json.join(", ")
                )
            }
            IrExpr::FieldAccess { target, field } => {
                // Strip dunder prefix for Rust struct field (Python private -> Rust private convention)
                let rust_field = field.trim_start_matches("__");
                format!(
                    "{}.{}",
                    self.emit_expr_internal(target),
                    to_snake_case(rust_field)
                )
            }
            IrExpr::Reference { target } => {
                format!("&{}", self.emit_expr_internal(target))
            }
            IrExpr::MutReference { target } => {
                format!("&mut {}", self.emit_expr_internal(target))
            }
            IrExpr::Print { args } => {
                // Generate println! with type-aware formatting
                // Type::Any uses {} (Display), others use {:?} (Debug)
                if args.is_empty() {
                    "println!()".to_string()
                } else {
                    let format_specs: Vec<&str> = args
                        .iter()
                        .map(|(_, ty)| {
                            // Use {} for Type::Any (serde_json::Value) and Type::String
                            // to avoid escape characters and quotes
                            if matches!(ty, Type::Any | Type::String) {
                                "{}"
                            } else {
                                "{:?}"
                            }
                        })
                        .collect();
                    let format_string = format_specs.join(" ");

                    let arg_strs: Vec<String> = args
                        .iter()
                        .map(|(expr, ty)| {
                            let expr_str = self.emit_expr_internal(expr);
                            // For string literals, emit directly
                            if let IrExpr::StringLit(s) = expr {
                                format!("\"{s}\"")
                            } else if matches!(ty, Type::Any) {
                                // For Type::Any (serde_json::Value), use display_value helper
                                // to handle Value::String without quotes
                                format!(
                                    "bridge::display_value(&{})",
                                    expr_str.trim_start_matches('&')
                                )
                            } else if expr_str.starts_with('&') {
                                expr_str
                            } else {
                                format!("&{expr_str}")
                            }
                        })
                        .collect();

                    format!("println!(\"{}\", {})", format_string, arg_strs.join(", "))
                }
            }
            // V1.3.1: StructConstruct - semantic now provides field information
            IrExpr::StructConstruct { name, fields } => {
                let field_inits: Vec<String> = fields
                    .iter()
                    .map(|(field_name, value)| {
                        format!(
                            "{}: {}",
                            to_snake_case(field_name),
                            self.emit_expr_no_outer_parens(value)
                        )
                    })
                    .collect();
                format!("{} {{ {} }}", name, field_inits.join(", "))
            }
            // V1.6.0: DynamicWrap - wrap value in enum variant
            IrExpr::DynamicWrap {
                enum_name,
                variant,
                value,
            } => {
                format!("{}::{}({})", enum_name, variant, self.emit_expr(value))
            }
            IrExpr::Unwrap(inner) => {
                format!("{}.unwrap()", self.emit_expr_internal(inner))
            }
            IrExpr::BridgeMethodCall {
                target,
                method,
                args,
            } => {
                self.needs_resident = true;
                // V1.5.2: Use ? for error propagation
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;

                let target_str = self.emit_expr(target);
                let args_str: Vec<_> = args
                    .iter()
                    .map(|a| {
                        if matches!(a, IrExpr::NoneLit) {
                            "tsuchinoko::bridge::protocol::TnkValue::Value { value: None }".to_string()
                        } else {
                            format!(
                                "tsuchinoko::bridge::protocol::TnkValue::from({})",
                                self.emit_expr_no_outer_parens(a)
                            )
                        }
                    })
                    .collect();

                format!(
                    "py_bridge.call_method(&{}, \"{}\", &[{}]).map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                    target_str,
                    method,
                    args_str.join(", ")
                )
            }
            IrExpr::BridgeAttributeAccess { target, attribute } => {
                self.needs_resident = true;
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;
                let target_str = self.emit_expr(target);
                format!(
                    "py_bridge.get_attribute(&{}, \"{}\").map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                    target_str, attribute
                )
            }
            IrExpr::BridgeItemAccess { target, index } => {
                self.needs_resident = true;
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;
                let target_str = self.emit_expr(target);
                let index_str = if matches!(index.as_ref(), IrExpr::NoneLit) {
                    "tsuchinoko::bridge::protocol::TnkValue::Value { value: None }".to_string()
                } else {
                    format!(
                        "tsuchinoko::bridge::protocol::TnkValue::from({})",
                        self.emit_expr(index)
                    )
                };

                format!(
                    "py_bridge.get_item(&{}, &{}).map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                    target_str, index_str
                )
            }
            IrExpr::BridgeSlice {
                target,
                start,
                stop,
                step,
            } => {
                self.needs_resident = true;
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;
                let target_str = self.emit_expr(target);

                let start_str = if matches!(start.as_ref(), IrExpr::NoneLit) {
                    "tsuchinoko::bridge::protocol::TnkValue::Value { value: None }".to_string()
                } else {
                    format!(
                        "tsuchinoko::bridge::protocol::TnkValue::from({})",
                        self.emit_expr(start)
                    )
                };
                let stop_str = if matches!(stop.as_ref(), IrExpr::NoneLit) {
                    "tsuchinoko::bridge::protocol::TnkValue::Value { value: None }".to_string()
                } else {
                    format!(
                        "tsuchinoko::bridge::protocol::TnkValue::from({})",
                        self.emit_expr(stop)
                    )
                };
                let step_str = if matches!(step.as_ref(), IrExpr::NoneLit) {
                    "tsuchinoko::bridge::protocol::TnkValue::Value { value: None }".to_string()
                } else {
                    format!(
                        "tsuchinoko::bridge::protocol::TnkValue::from({})",
                        self.emit_expr(step)
                    )
                };

                format!(
                    "py_bridge.slice(&{}, {}, {}, {}).map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                    target_str, start_str, stop_str, step_str
                )
            }
            IrExpr::PyO3Call {
                module,
                method,
                args,
            } => {
                // エイリアス → 実モジュール名に変換（静的マッピング）
                let real_module = match module.as_str() {
                    "np" => "numpy".to_string(),
                    "pd" => "pandas".to_string(),
                    _ => {
                        // external_imports から逆引き
                        self.external_imports
                            .iter()
                            .find(|(_, alias)| alias == module)
                            .map(|(real, _)| real.clone())
                            .unwrap_or_else(|| module.clone())
                    }
                };

                let target = format!("{real_module}.{method}");
                let mut arg_evals = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    arg_evals.push(format!(
                        "let _arg_{} = {};",
                        i,
                        self.emit_expr_internal(arg)
                    ));
                }

                let args_str: Vec<String> = (0..args.len()).map(|i| format!("_arg_{i}")).collect();

                // 方式選択テーブルを参照
                use crate::bridge::module_table::{
                    generate_native_code, get_import_mode, ImportMode,
                };

                match get_import_mode(&target) {
                    ImportMode::Native => {
                        // Rust ネイティブ実装 - PyO3/Resident 不要
                        // Native の場合は、既に arg_evals があると困るかもしれないが、
                        // Expression block にすれば OK
                        let native_code = generate_native_code(&target, &args_str)
                            .unwrap_or_else(|| format!("/* Native not implemented: {target} */"));
                        format!(
                            "{{\n{}    {}\n}}",
                            arg_evals.join("\n    ") + "\n",
                            native_code
                        )
                    }
                    ImportMode::PyO3 | ImportMode::Resident => {
                        // 常駐プロセス方式が必要
                        self.needs_resident = true;
                        let args_json: Vec<String> = (0..args.len())
                            .map(|i| format!("serde_json::json!(_arg_{i})"))
                            .collect();
                        // V1.5.2: Use ? instead of unwrap() for error propagation
                        // Mark that this function now may raise (for Ok() wrapping of returns)
                        self.current_func_may_raise = true;
                        self.uses_tsuchinoko_error = true;
                        format!(
                            "{{\n{}    py_bridge.call_json::<tsuchinoko::bridge::protocol::TnkValue>(\"{}\", &[{}]).map_err(|e| TsuchinokoError::new(\"ExternalError\", &e, None))?\n}}",
                            arg_evals.join("\n    ") + "\n",
                            target,
                            args_json.join(", ")
                        )
                    }
                }
            }
        }
    }

    /// Emit expression without outer parentheses (for if/while conditions)
    fn emit_expr_no_outer_parens(&mut self, expr: &IrExpr) -> String {
        let s = self.emit_expr(expr);
        if s.starts_with('(') && s.ends_with(')') {
            // Check if these are matching outer parens
            let inner = &s[1..s.len() - 1];
            // Simple check: if inner has balanced parens, strip outer
            let mut depth = 0;
            let mut valid = true;
            for c in inner.chars() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth < 0 {
                            valid = false;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if valid && depth == 0 {
                return inner.to_string();
            }
        }
        s
    }
}

/// Implementation of CodeEmitter trait for RustEmitter
impl CodeEmitter for RustEmitter {
    fn emit_node(&mut self, node: &IrNode) -> String {
        // Delegate to the internal implementation
        RustEmitter::emit_node_internal(self, node)
    }

    fn emit_expr(&mut self, expr: &IrExpr) -> String {
        // Delegate to the internal implementation
        RustEmitter::emit_expr_internal(self, expr)
    }
}

#[cfg(test)]
mod tests;
