//! Emitter module - Rust code generation

use crate::ir::{HoistedVar, IrAugAssignOp, IrBinOp, IrExpr, IrExprKind, IrNode, IrUnaryOp};
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
    // uses_pyo3: bool, // Removed as unused
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
    /// V1.5.2: Whether we are inside except body
    in_except_body: bool,
    /// V1.5.2: Current except variable name (if any)
    current_except_var: Option<String>,
    /// V1.7.0: Current function's return type (for Return in Try handling)
    current_ret_type: Option<Type>,
    /// V1.6.0: Map of struct name -> base class name (for composition)
    struct_bases: HashMap<String, String>,
}

/// Convert camelCase/PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    if s == "_" {
        return "__tnk_underscore".to_string();
    }
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
            // uses_pyo3: false, // Removed as unused
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
            in_except_body: false,
            current_except_var: None,
            current_ret_type: None,
            struct_bases: HashMap::new(),
        }
    }

    pub fn emit_nodes(&mut self, nodes: &[IrNode]) -> String {
        let old_shadowed_len = self.shadowed_vars.len();
        // If we're at indent 0, this is a top-level call
        let is_top_level = self.indent == 0;

        // Pass 1: Collect all PyO3Import nodes first (top-level only)
        if is_top_level {
            for node in nodes {
                if let IrNode::BridgeImport {
                    module,
                    alias,
                    items: _,
                } = node
                {
                    // Bridge imports are now handled dynamically
                    if !self.external_imports.iter().any(|(m, _)| m == module) {
                         // We don't emit anything in pre-pass for BridgeImport
                         // Logic is handled in emit_node
                         // But we might want to track them.
                         // For now, just ensure the module is in external_imports for resident_wrapped
                         let effective_alias = alias.clone().unwrap_or_else(|| module.clone());
                         self.external_imports.push((module.clone(), effective_alias));
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
        self.shadowed_vars.truncate(old_shadowed_len);

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
            } else {
                // V1.7.0: Add standalone stubs for TnkValue if needed
                self.prepend_standalone_stubs(&final_body)
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
use tsuchinoko::bridge::protocol::{{TnkValue, DictItem}};
            
{body}

// Note: This code uses the PythonBridge for calling Python libraries.
// Make sure Python is installed and the required libraries are available.
// The Python worker process will be started automatically.
"#
        )
    }

    /// V1.7.0: Add minimal TnkValue stub for standalone builds that might reference it
    fn prepend_standalone_stubs(&self, body: &str) -> String {
        // If the code references TnkValue but doesn't use the resident bridge,
        // we provide a minimal stub to allow standalone rustc builds to pass.
        if body.contains("TnkValue") && !body.contains("PythonBridge") {
            format!(
                r#"#[allow(dead_code)]
#[derive(Clone, Debug)]
enum TnkValue {{
    Value {{ value: Option<String> }},
    Dict {{ items: Vec<DictItem> }},
    List {{ items: Vec<TnkValue> }},
    Handle {{ id: String, session: String }},
}}
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct DictItem {{ key: TnkValue, value: TnkValue }}
impl From<i64> for TnkValue {{ fn from(i: i64) -> Self {{ TnkValue::Value {{ value: Some(i.to_string()) }} }} }}
impl From<f64> for TnkValue {{ fn from(f: f64) -> Self {{ TnkValue::Value {{ value: Some(f.to_string()) }} }} }}
impl From<bool> for TnkValue {{ fn from(b: bool) -> Self {{ TnkValue::Value {{ value: Some(b.to_string()) }} }} }}
impl From<String> for TnkValue {{ fn from(s: String) -> Self {{ TnkValue::Value {{ value: Some(s) }} }} }}
impl From<&str> for TnkValue {{ fn from(s: &str) -> Self {{ TnkValue::Value {{ value: Some(s.to_string()) }} }} }}
impl<T: Into<TnkValue>> From<Vec<T>> for TnkValue {{ fn from(v: Vec<T>) -> Self {{ TnkValue::List {{ items: v.into_iter().map(|e| e.into()).collect() }} }} }}
impl<K: Into<TnkValue>, V: Into<TnkValue>> From<std::collections::HashMap<K, V>> for TnkValue {{
    fn from(m: std::collections::HashMap<K, V>) -> Self {{
        TnkValue::Dict {{ items: m.into_iter().map(|(k, v)| DictItem {{ key: k.into(), value: v.into() }}).collect() }}
    }}
}}
impl TnkValue {{
    pub fn is_none(&self) -> bool {{ matches!(self, TnkValue::Value {{ value: None }}) }}
    pub fn as_i64(&self) -> Option<i64> {{ if let TnkValue::Value {{ value: Some(s) }} = self {{ s.parse().ok() }} else {{ None }} }}
    pub fn as_f64(&self) -> Option<f64> {{ if let TnkValue::Value {{ value: Some(s) }} = self {{ s.parse().ok() }} else {{ None }} }}
    pub fn as_bool(&self) -> Option<bool> {{ if let TnkValue::Value {{ value: Some(s) }} = self {{ s.parse().ok() }} else {{ None }} }}
    pub fn as_str(&self) -> Option<&str> {{ if let TnkValue::Value {{ value: Some(s) }} = self {{ Some(s.as_str()) }} else {{ None }} }}
}}
impl std::fmt::Display for TnkValue {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        match self {{
            TnkValue::Value {{ value: Some(s) }} => write!(f, "{{}}", s),
            TnkValue::Value {{ value: None }} => write!(f, "None"),
            TnkValue::Dict {{ items }} => {{
                write!(f, "{{{{")?;
                for (i, item) in items.iter().enumerate() {{
                    if i > 0 {{ write!(f, ", ")?; }}
                    write!(f, "{{}}: {{}}", item.key, item.value)?;
                }}
                write!(f, "}}}}")
            }}
            TnkValue::List {{ items }} => {{
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {{
                    if i > 0 {{ write!(f, ", ")?; }}
                    write!(f, "{{}}", item)?;
                }}
                write!(f, "]")
            }}
            TnkValue::Handle {{ id, .. }} => write!(f, "<Handle:{{}}>", id),
        }}
    }}
}}

{body}"#
            )
        } else {
            body.to_string()
        }
    }

    /// V1.7.0: helper to emit expression as TnkValue, handling recursive Dicts (Moved to inherent impl)
    fn emit_as_tnk_value(&mut self, expr: &IrExpr) -> String {
        match &expr.kind {
             IrExprKind::Dict { entries, .. } => {
                let items_str = entries.iter().map(|(k, v)| {
                    format!(
                        "DictItem {{ key: {}, value: {} }}",
                        self.emit_as_tnk_value(k),
                        self.emit_as_tnk_value(v)
                    )
                }).collect::<Vec<_>>().join(", ");
                format!("TnkValue::Dict {{ items: vec![{}] }}", items_str)
            }
             IrExprKind::List { elements, .. } => {
                 let elems_str = elements.iter().map(|e| self.emit_as_tnk_value(e)).collect::<Vec<_>>().join(", ");
                 format!("TnkValue::List {{ items: vec![{}] }}", elems_str)
             }
            IrExprKind::Ref(inner) => self.emit_as_tnk_value(inner),
            IrExprKind::Var(_) | IrExprKind::BridgeGet { .. } => format!("TnkValue::from({}.clone())", self.emit_expr(expr)),
            _ => format!("TnkValue::from({})", self.emit_expr(expr)),
        }
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
                let is_hoisted = self
                    .current_hoisted_vars
                    .iter()
                    .any(|v| to_snake_case(&v.name) == snake_name);

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
                                && matches!(expr.kind, IrExprKind::StringLit(_))
                            {
                                if let IrExprKind::StringLit(s) = &expr.kind {
                                    format!("\"{s}\".to_string()")
                                } else {
                                    self.emit_expr_no_outer_parens(expr)
                                }
                            } else if matches!(expr.kind, IrExprKind::Tuple(_)) {
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
                for (name, _, _) in targets {
                    let snake_name = to_snake_case(name);
                    let is_hoisted = self
                        .current_hoisted_vars
                        .iter()
                        .any(|v| to_snake_case(&v.name) == snake_name);
                    if is_hoisted {
                        self.shadowed_vars.push(snake_name);
                    }
                }
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
                needs_bridge,
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

                    // Set current hoisted variables for top-level scope
                    let old_hoisted =
                        std::mem::replace(&mut self.current_hoisted_vars, hoisted_vars.clone());

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

                    let body_str = self.emit_nodes(body);

                    // Restore may_raise state
                    self.current_func_may_raise = old_may_raise;
                    self.current_hoisted_vars = old_hoisted;

                    // 関数内で resident 機能が使われたか
                    let func_needs_resident = self.needs_resident;

                    // グローバルステートを復元（OR演算）
                    self.needs_resident = needs_resident_backup || func_needs_resident;

                    self.indent -= 1;

                    let body_str = if hoisted_decls.is_empty() {
                        body_str
                    } else {
                        format!("{hoisted_decls}\n{body_str}")
                    };

                    if func_needs_resident || self.needs_resident {
                        // self.needs_resident is global state (might be set by previous nodes)
                        self.emit_resident_main(&body_str)
                    } else {
                        // V1.5.2: Wrap main in catch_unwind for panic diagnosis
                        format!(
                            r#"fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), Box<dyn std::error::Error>> {{
{body_str}
        Ok(())
    }}));
    match result {{
        Ok(inner_result) => inner_result,
        Err(e) => {{
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
    }}
}}"#
                        )
                    }
                } else {
                    let snake_name = if name == "main" {
                        // Rename user's 'main' to _main_tsuchinoko to avoid conflict with Rust's entry point
                        "_main_tsuchinoko".to_string()
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

                    // V1.7.0: Set return type for TryBlock return value type inference
                    let old_ret_type = self.current_ret_type.clone();
                    self.current_ret_type = Some(ret.clone());

                    let body_str = self.emit_nodes(body);
                    self.indent -= 1;

                    // Restore previous hoisted vars, may_raise and ret_type
                    self.current_hoisted_vars = old_hoisted;
                    self.current_func_may_raise = old_may_raise;
                    self.current_ret_type = old_ret_type;

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
                        let backup_hoisted =
                            std::mem::replace(&mut self.current_hoisted_vars, hoisted_vars.clone());
                        // ONLY set this if we are NOT in the special __top_level__ (fn main)
                        // Actually, this block is the "else" (non-__top_level__) path, so it's always true.
                        self.is_inside_resident_func = true;

                        // Phase F: Set may_raise for proper Ok() wrapping in Return statements
                        let backup_may_raise = self.current_func_may_raise;
                        self.current_func_may_raise = *may_raise || func_needs_resident;
                        self.current_ret_type = Some(ret.clone()); // Store ret type for inner Try blocks

                        // Reset needs_resident just in case, though we know it will become true
                        self.needs_resident = false;
                        let s = self.emit_nodes(body);

                        self.is_inside_resident_func = backup_flag;
                        self.current_hoisted_vars = backup_hoisted;
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

                    if *needs_bridge || func_needs_resident {
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
                // V1.5.2: Inside try body, we can't use 'return val' directly because we are in a closure.
                // We use __ret_val if it was hoisted, or rely on Tsuchinoko's TryBlock handling.
                if self.in_try_body {
                    match expr {
                        Some(e) => {
                            // If we have a return value, we MUST store it somewhere.
                            // The specialized TryBlock IR would have already detected this?
                            // For now, let's assume __ret_val is available if needed.
                            return format!("{}__ret_val = Some({}); return Ok(());", indent, self.emit_expr(e));
                        }
                        None => {
                             return format!("{}return Ok(());", indent);
                        }
                    }
                }

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
                if let IrExprKind::StringLit(s) = &expr.kind {
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

                // V1.5.2: If try body (or except/else) has Return, we need __ret_val.
                // We recursively check for IrNode::Return in bodies.
                fn has_return(nodes: &[IrNode]) -> bool {
                    nodes.iter().any(|n| match n {
                        IrNode::Return(_) => true,
                        IrNode::If { then_block, else_block, .. } => has_return(then_block) || else_block.as_ref().map(|b| has_return(b)).unwrap_or(false),
                        IrNode::TryBlock { try_body, except_body, else_body, finally_body, .. } => 
                            has_return(try_body) || has_return(except_body) || 
                            else_body.as_ref().map(|b| has_return(b)).unwrap_or(false) ||
                            finally_body.as_ref().map(|b| has_return(b)).unwrap_or(false),
                        _ => false,
                    })
                }
                
                let try_has_return = has_return(try_body) || has_return(except_body) || else_body.as_ref().map(|b| has_return(&b)).unwrap_or(false);
                if try_has_return {
                    let ret_ty_str = self.current_ret_type.as_ref().map(|t: &crate::semantic::Type| t.to_rust_string()).unwrap_or_else(|| "TnkValue".to_string());
                    result.push_str(&format!("{indent}let mut __ret_val: Option<{}> = None;\n", ret_ty_str));
                }

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
                    "{indent}let __try_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), Box<dyn std::error::Error>> {{\n"
                ));
                self.indent += 1;

                // Set hoisted vars for unwrap() in try body (for return statements)
                let old_try_hoisted_vars = std::mem::replace(
                    &mut self.try_hoisted_vars,
                    hoisted_vars
                        .iter()
                        .map(|(name, _)| to_snake_case(name))
                        .collect(),
                );

                // V1.5.2: Set in_try_body flag - closure allows ? now
                let old_in_try_body = self.in_try_body;
                self.in_try_body = true;

                // Emit try body
                for node in try_body {
                    if need_hoisting {
                        if let IrNode::VarDecl {
                            name,
                            init: Some(expr),
                            ..
                        } = node
                        {
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

                    result.push_str(&self.emit_node(node));
                    result.push('\n');
                }
                
                self.indent -= 1;
                self.in_try_body = old_in_try_body;
                self.try_hoisted_vars = old_try_hoisted_vars;
                
                // End closure with Ok(())
                result.push_str(&format!("{indent}    Ok(())\n"));
                result.push_str(&format!("{indent}}}));\n"));

                // Handle result: match on outer (panic) and inner (exception)
                // If OK -> Else block
                // If Err -> Except block
                result.push_str(&format!("{indent}match __try_result {{\n"));
                result.push_str(&format!("{indent}    Ok(Ok(_)) => {{\n")); // Success

                // Handle return if occurred inside try
                if try_has_return {
                    let inner_indent = "    ".repeat(self.indent + 1);
                    if self.current_func_may_raise {
                         result.push_str(&format!("{}if let Some(val) = __ret_val {{ return Ok(val); }}\n", inner_indent));
                    } else {
                         result.push_str(&format!("{}if let Some(val) = __ret_val {{ return val; }}\n", inner_indent));
                    }
                }
                
                // Else block logic
                if let Some(else_nodes) = else_body {
                    self.indent += 2;
                    let old_try_hoisted = std::mem::replace(
                        &mut self.try_hoisted_vars,
                        hoisted_vars.iter().map(|(n,_)| to_snake_case(n)).collect()
                    );
                    for node in else_nodes {
                         result.push_str(&self.emit_node(node));
                         result.push('\n');
                    }
                    self.try_hoisted_vars = old_try_hoisted;
                    self.indent -= 2;
                }
                result.push_str(&format!("{indent}    }}\n")); 
                
                // Error handling logic (Exception OR Panic) 
                // We combine both cases to run except_block
                result.push_str(&format!("{indent}    Ok(Err(__exc)) => {{\n")); // Python Exception
                if let Some(var_name) = except_var {
                     self.indent += 2;
                     // Bind exception
                     if self.current_func_may_raise {
                         result.push_str(&format!(
                             "{}let {} = TsuchinokoError::new(\"Exception\", &format!(\"{{:?}}\", __exc), None);\n",
                             "    ".repeat(self.indent), to_snake_case(var_name)
                         ));
                     } else {
                         // Fallback string binding
                         result.push_str(&format!(
                            "{}let {} = format!(\"{{:?}}\", __exc);\n",
                             "    ".repeat(self.indent), to_snake_case(var_name)
                         ));
                     }
                     self.indent -= 2;
                }
                // Emit Except Body
                if !except_body.is_empty() {
                     let old_in_except_body = self.in_except_body;
                     let old_except_var = self.current_except_var.clone();
                     self.in_except_body = true;
                     self.current_except_var = except_var.clone();
                     self.indent += 2;
                     let old_try_hoisted_except = std::mem::replace(
                        &mut self.try_hoisted_vars,
                        hoisted_vars.iter().map(|(n,_)| to_snake_case(n)).collect()
                    );
                     for node in except_body {
                         result.push_str(&self.emit_node(node));
                         result.push('\n');
                     }
                     self.try_hoisted_vars = old_try_hoisted_except;
                     self.indent -= 2;
                     self.in_except_body = old_in_except_body;
                     self.current_except_var = old_except_var;
                }
                result.push_str(&format!("{indent}    }}\n"));

                // Panic case
                result.push_str(&format!("{indent}    Err(__panic) => {{\n"));
                if let Some(var_name) = except_var {
                     self.indent += 2;
                     let indent_str = "    ".repeat(self.indent);
                     // Bind panic
                     result.push_str(&format!(
                         "{}let {}: String = if let Some(s) = __panic.downcast_ref::<&str>() {{ s.to_string() }} else if let Some(s) = __panic.downcast_ref::<String>() {{ s.clone() }} else {{ \"Unknown panic\".to_string() }};\n",
                         indent_str, to_snake_case(var_name)
                     ));
                     // If TsuchinokoError is needed
                     if self.current_func_may_raise {
                         result.push_str(&format!(
                             "{}let {} = TsuchinokoError::new(\"InternalError\", {}, None);\n",
                             indent_str, to_snake_case(var_name), to_snake_case(var_name)
                         ));
                     }
                     self.indent -= 2;
                }
                 // Emit Except Body (Duplicate) - Ideally functionize this, but copy-paste is safer for now to ensure context
                if !except_body.is_empty() {
                     let old_in_except_body = self.in_except_body;
                     let old_except_var = self.current_except_var.clone();
                     self.in_except_body = true;
                     self.current_except_var = except_var.clone();
                     self.indent += 2;
                     let old_try_hoisted_except = std::mem::replace(
                        &mut self.try_hoisted_vars,
                        hoisted_vars.iter().map(|(n,_)| to_snake_case(n)).collect()
                    );
                     for node in except_body {
                         result.push_str(&self.emit_node(node));
                         result.push('\n');
                     }
                     self.try_hoisted_vars = old_try_hoisted_except;
                     self.indent -= 2;
                     self.in_except_body = old_in_except_body;
                     self.current_except_var = old_except_var;
                }
                result.push_str(&format!("{indent}    }}\n"));
                
                result.push_str(&format!("{indent}}};\n")); // End match



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

                // V1.7.0: Add a fallback return if we fell through the TryBlock and the function expects a value
                if let Some(ret_ty) = &self.current_ret_type {
                    if *ret_ty != Type::Unit {
                        if self.current_func_may_raise {
                            result.push_str(&format!("{}return Ok({});\n", indent, ret_ty.to_default_value()));
                        } else {
                            result.push_str(&format!("{}return {};\n", indent, ret_ty.to_default_value()));
                        }
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
                needs_bridge,
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

                // V1.7.0: Use needs_bridge to decide if py_bridge argument is needed
                let params_str = if *needs_bridge {
                    let mut p = params_str;
                    p.insert(0, "py_bridge: &mut tsuchinoko::bridge::PythonBridge".to_string());
                    p.join(", ")
                } else {
                    params_str.join(", ")
                };

                let mut result = format!(
                    "{}fn {}({}{}){} {{\n",
                    inner_indent,
                    to_snake_case(name),
                    self_param,
                    params_str,
                    ret_str
                );

                // V1.5.2: Track may_raise for return statement wrapping
                let old_may_raise = self.current_func_may_raise;
                self.current_func_may_raise = *may_raise;

                // V1.7.0: Set return type for TryBlock return value type inference
                let old_ret_type = self.current_ret_type.clone();
                self.current_ret_type = Some(ret.clone());

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
                self.current_ret_type = old_ret_type;
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
                if exc_type.is_empty() {
                    if self.in_except_body {
                        if let Some(var) = &self.current_except_var {
                            let var_name = to_snake_case(var);
                            if self.current_func_may_raise {
                                return format!("{indent}return Err({var_name}.into());");
                            }
                            return format!("{indent}panic!(\"{{}}\", {var_name});");
                        }
                        return format!("{indent}panic!(\"Invalid bare raise\");");
                    }
                    return format!("{indent}panic!(\"Invalid bare raise\");");
                }
                let msg_str = self.emit_expr(message);

                // V1.5.2: Inside try body, use panic! so catch_unwind can catch it
                if self.in_try_body {
                    // Inside try block: use panic! for catch_unwind to catch
                    format!("{indent}panic!(\"[{}] {{}}\", {});", exc_type, msg_str)
                } else {
                    // Outside try block: generate Err(TsuchinokoError::...)
                    match cause {
                        Some(cause_expr) => {
                            // With cause: Err(TsuchinokoError::with_line("Type", "msg", line, Some(cause)).into())
                            format!(
                                "{indent}return Err(TsuchinokoError::with_line(\"{}\", &format!(\"{{}}\", {}), {}, Some({})).into());",
                                exc_type,
                                msg_str,
                                line,
                                self.emit_expr(cause_expr)
                            )
                        }
                        None => {
                            // Without cause: Err(TsuchinokoError::with_line("Type", "msg", line, None).into())
                            format!(
                                "{indent}return Err(TsuchinokoError::with_line(\"{}\", &format!(\"{{}}\", {}), {}, None).into());",
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
            IrNode::BridgeImport {
                module,
                alias,
                items,
            } => {
                // V1.7.0: Emit Bridge import code to bind module handle to variable
                self.needs_resident = true;
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;

                if let Some(ref item_list) = items {
                     // from module import a, b
                     let mut code = String::new();
                     for item in item_list {
                         code.push_str(&format!(
                            "py_bridge.import(\"{}.{}\", \"{}\");\n",
                            module, item, item
                         ));
                     }
                     code
                } else {
                    // import module as alias
                    let import_name = module.clone();
                    let var_name = alias.clone().unwrap_or(module.clone());
                    
                    // V1.7.0: Register in ModuleTable
                    format!(
                        "py_bridge.import(\"{}\", \"{}\");\n",
                        import_name, var_name
                    )
                }

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
        match &expr.kind {
            IrExprKind::BuiltinCall { id, args } => {
                let func_name = id.to_rust_name();
                let args_str = args
                    .iter()
                    .map(|arg| self.emit_expr_internal(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", func_name, args_str)
            }
            IrExprKind::FromTnkValue { value, to_type } => {
                let value_str = self.emit_expr_internal(value);
                let base = if value_str.ends_with('?') {
                    format!("({})", value_str)
                } else {
                    value_str
                };
                let use_fallible = self.current_func_may_raise;
                if use_fallible {
                    self.uses_tsuchinoko_error = true;
                }
                match to_type {
                    Type::Int => {
                        if use_fallible {
                            format!("{}.as_i64().ok_or_else(|| TsuchinokoError::internal(\"TnkValue to i64 failed\"))?", base)
                        } else {
                            format!("{}.as_i64().unwrap()", base)
                        }
                    }
                    Type::Float => {
                        if use_fallible {
                            format!("{}.as_f64().ok_or_else(|| TsuchinokoError::internal(\"TnkValue to f64 failed\"))?", base)
                        } else {
                            format!("{}.as_f64().unwrap()", base)
                        }
                    }
                    Type::Bool => {
                        if use_fallible {
                            format!("{}.as_bool().ok_or_else(|| TsuchinokoError::internal(\"TnkValue to bool failed\"))?", base)
                        } else {
                            format!("{}.as_bool().unwrap()", base)
                        }
                    }
                    Type::String => {
                        if use_fallible {
                            format!("{}.as_str().ok_or_else(|| TsuchinokoError::internal(\"TnkValue to String failed\"))?.to_string()", base)
                        } else {
                            format!("{}.as_str().unwrap().to_string()", base)
                        }
                    }
                    Type::Optional(inner) => {
                        let inner_str = match inner.as_ref() {
                            Type::Int => {
                                if use_fallible {
                                    "__v.as_i64().ok_or_else(|| TsuchinokoError::internal(\"TnkValue to i64 failed\"))?".to_string()
                                } else {
                                    "__v.as_i64().unwrap()".to_string()
                                }
                            }
                            Type::Float => {
                                if use_fallible {
                                    "__v.as_f64().ok_or_else(|| TsuchinokoError::internal(\"TnkValue to f64 failed\"))?".to_string()
                                } else {
                                    "__v.as_f64().unwrap()".to_string()
                                }
                            }
                            Type::Bool => {
                                if use_fallible {
                                    "__v.as_bool().ok_or_else(|| TsuchinokoError::internal(\"TnkValue to bool failed\"))?".to_string()
                                } else {
                                    "__v.as_bool().unwrap()".to_string()
                                }
                            }
                            Type::String => {
                                if use_fallible {
                                    "__v.as_str().ok_or_else(|| TsuchinokoError::internal(\"TnkValue to String failed\"))?.to_string()".to_string()
                                } else {
                                    "__v.as_str().unwrap().to_string()".to_string()
                                }
                            }
                            Type::Any | Type::Unknown => "__v".to_string(),
                            _ => "__v".to_string(),
                        };
                        format!(
                            "({{ let __v = {}; if __v.is_none() {{ None }} else {{ Some({}) }} }})",
                            base, inner_str
                        )
                    }
                    Type::Any | Type::Unknown => base,
                    _ => base,
                }
            }
            IrExprKind::IntLit(n) => format!("{n}i64"),
            IrExprKind::FloatLit(f) => format!("{f:.1}"),
            IrExprKind::StringLit(s) => format!("\"{s}\""),
            IrExprKind::BoolLit(b) => b.to_string(),
            IrExprKind::NoneLit => "None".to_string(),
            IrExprKind::Var(name) => {
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
            IrExprKind::BinOp { left, op, right } => {
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
                            "py_bridge.call_json::<TnkValue>(\"numpy.matmul\", &[serde_json::json!({}), serde_json::json!({})]).map_err(|e| TsuchinokoError::new(\"ExternalError\", &e, None))?",
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
            IrExprKind::UnaryOp { op, operand } => {
                let op_str = match op {
                    IrUnaryOp::Neg => "-",
                    IrUnaryOp::Not => "!",
                    IrUnaryOp::Deref => "*",
                    IrUnaryOp::BitNot => "!", // V1.3.0 - Rust uses ! for bitwise NOT too
                };
                format!("({}{})", op_str, self.emit_expr(operand))
            }
            IrExprKind::Call {
                func,
                args,
                callee_may_raise,
                callee_needs_bridge,
            } => {
                let is_print = if let IrExprKind::Var(name) = &func.kind {
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
                            let unwrapped = match &a.kind {
                                IrExprKind::MethodCall {
                                    target,
                                    method,
                                    args: mc_args,
                                    target_type: _,
                                    callee_needs_bridge: _,
                                } if mc_args.is_empty()
                                    && (method == "clone" || method == "to_string") =>
                                {
                                    target.as_ref()
                                }
                                _ => a,
                            };

                            // For string literals, emit directly
                            match &unwrapped.kind {
                                IrExprKind::StringLit(s) => format!("\"{s}\""),
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
                    let func_name_opt = if let IrExprKind::Var(name) = &func.kind {
                        Some(name.clone())
                    } else {
                        None
                    };

                    if let Some(name) = func_name_opt {
                        // V1.4.0: Check if this is a from-imported function
                        // external_imports contains (module, item) tuples
                        // If name matches any item, convert to py_bridge.call_json("module.item", ...)
                        // V1.4.0: Check if this is a resident (bridge) function
                        // Handled by IrExpr::BridgeCall now. 
                        // If it fell through here, it's a regular Rust call.

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
                                "_main_tsuchinoko".to_string()
                            } else {
                                to_snake_case(&name)
                            };
                            // V1.7.0: Use callee_needs_bridge flag
                            if *callee_needs_bridge || self.resident_functions.contains(&func_name) {
                                self.needs_resident = true;
                                if self.indent > 0 && name != "main" && name != "__top_level__" {
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
                        let needs_parens = matches!(&func.kind, IrExprKind::FieldAccess { .. });
                        if needs_parens {
                            format!("({})({})", func_str, args_str.join(", "))
                        } else {
                            format!("/* IrExprKind::Call */ {}({})", func_str, args_str.join(", "))
                        }
                    }
                }
            }
            IrExprKind::StaticCall { path, args } => {
                let args_str = args
                    .iter()
                    .map(|a| self.emit_expr_no_outer_parens(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{path}({args_str})")
            }
            IrExprKind::List {
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
            IrExprKind::Dict {
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
            IrExprKind::Set {
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
            IrExprKind::FString { parts, values } => {
                let mut format_str = String::new();
                for (i, part) in parts.iter().enumerate() {
                    format_str.push_str(part);
                    if i < values.len() {
                        let (_, ty) = &values[i];
                        if is_display_compatible(ty) {
                            format_str.push_str("{}");
                        } else {
                            format_str.push_str("{:?}");
                        }
                    }
                }

                let value_strs: Vec<_> = values
                    .iter()
                    .map(|(v, ty)| {
                        let s = self.emit_expr_internal(v);
                        if is_any_type(ty) {
                            // Use display_value helper if available
                            if self.needs_resident {
                                format!("bridge::display_value(&{})", s.trim_start_matches('&'))
                            } else {
                                s
                            }
                        } else {
                            s
                        }
                    })
                    .collect();

                if values.is_empty() {
                    format!("\"{}\"", parts.join(""))
                } else {
                    format!("format!(\"{}\", {})", format_str, value_strs.join(", "))
                }
            }
            IrExprKind::IfExp { test, body, orelse } => {
                format!(
                    "if {} {{ {} }} else {{ {} }}",
                    self.emit_expr_internal(test),
                    self.emit_expr_internal(body),
                    self.emit_expr_internal(orelse)
                )
            }
            IrExprKind::ListComp {
                elt,
                target,
                iter,
                condition,
            } => {
                let old_shadowed_len = self.shadowed_vars.len();
                // Use .iter().cloned() to avoid ownership transfer
                // This allows the same collection to be used multiple times

                let target_has_comma = target.contains(',');
                let target_snake = if target_has_comma {
                    let parts: Vec<String> =
                        target.split(',').map(|s| to_snake_case(s.trim())).collect();
                    format!("({})", parts.join(", "))
                } else {
                    to_snake_case(target)
                };
                if target_has_comma {
                    for part in target.split(',') {
                        self.shadowed_vars.push(to_snake_case(part.trim()));
                    }
                } else {
                    self.shadowed_vars.push(target_snake.clone());
                }

                let elt_str = self.emit_expr_internal(elt);

                // For tuple unpacking, always use the target name to avoid partial usage check complexity
                let closure_var = if target_has_comma || elt_str.contains(&target_snake) {
                    target_snake.clone()
                } else {
                    "_".to_string()
                };

                let iter_str = self.emit_expr_internal(iter);

                let iter_chain = match &iter.kind {
                    // Range needs parentheses for method chaining: (1..10).filter(...)
                    IrExprKind::Range { .. } => format!("({iter_str})"),
                    // MethodCall to items() returns a Vec - use .into_iter() for ownership
                    IrExprKind::MethodCall { method, .. } if method == "items" => {
                        format!("{iter_str}.into_iter()")
                    }
                    // Already an iterator (MethodCall with iter/filter/map), use directly
                    IrExprKind::MethodCall { method, .. }
                        if method.contains("iter")
                            || method.contains("filter")
                            || method.contains("map") =>
                    {
                        iter_str
                    }
                    // Collection: use .iter().cloned() to borrow and copy values
                    _ => format!("{iter_str}.iter().cloned()"),
                };

                let out = if let Some(cond) = condition {
                    let cond_str = self.emit_expr_internal(cond);
                    // Use pattern without & for filter - references are handled by the condition
                    format!(
                        "{}.filter(|{}| {}).map(|{}| {}).collect::<Vec<_>>()",
                        iter_chain, &target_snake, cond_str, closure_var, elt_str
                    )
                } else {
                    format!("{iter_chain}.map(|{closure_var}| {elt_str}).collect::<Vec<_>>()")
                };
                self.shadowed_vars.truncate(old_shadowed_len);
                out
            }
            // V1.3.0: Dict comprehension {k: v for target in iter if condition}
            IrExprKind::DictComp {
                key,
                value,
                target,
                iter,
                condition,
            } => {
                let old_shadowed_len = self.shadowed_vars.len();

                let target_has_comma = target.contains(',');
                let target_snake = if target_has_comma {
                    let parts: Vec<String> =
                        target.split(',').map(|s| to_snake_case(s.trim())).collect();
                    format!("({})", parts.join(", "))
                } else {
                    to_snake_case(target)
                };
                if target_has_comma {
                    for part in target.split(',') {
                        self.shadowed_vars.push(to_snake_case(part.trim()));
                    }
                } else {
                    self.shadowed_vars.push(target_snake.clone());
                }

                let key_str = self.emit_expr_internal(key);
                let value_str = self.emit_expr_internal(value);

                let iter_str = self.emit_expr_internal(iter);

                let iter_chain = match &iter.kind {
                    IrExprKind::Range { .. } => format!("({iter_str})"),
                    IrExprKind::MethodCall { method, .. } if method == "items" => {
                        format!("{iter_str}.into_iter()")
                    }
                    IrExprKind::MethodCall { method, .. }
                        if method.contains("iter")
                            || method.contains("filter")
                            || method.contains("map") =>
                    {
                        iter_str
                    }
                    _ => format!("{iter_str}.iter().cloned()"),
                };

                let out = if let Some(cond) = condition {
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
                };
                self.shadowed_vars.truncate(old_shadowed_len);
                out
            }
            // V1.6.0: Set comprehension {x for target in iter if condition}
            IrExprKind::SetComp {
                elt,
                target,
                iter,
                condition,
            } => {
                let old_shadowed_len = self.shadowed_vars.len();

                let target_has_comma = target.contains(',');
                let target_snake = if target_has_comma {
                    let parts: Vec<String> =
                        target.split(',').map(|s| to_snake_case(s.trim())).collect();
                    format!("({})", parts.join(", "))
                } else {
                    to_snake_case(target)
                };
                if target_has_comma {
                    for part in target.split(',') {
                        self.shadowed_vars.push(to_snake_case(part.trim()));
                    }
                } else {
                    self.shadowed_vars.push(target_snake.clone());
                }

                let elt_str = self.emit_expr_internal(elt);

                let closure_var = if target_has_comma || elt_str.contains(&target_snake) {
                    target_snake.clone()
                } else {
                    "_".to_string()
                };

                let iter_str = self.emit_expr_internal(iter);

                let iter_chain = match &iter.kind {
                    IrExprKind::Range { .. } => format!("({iter_str})"),
                    IrExprKind::MethodCall { method, .. }
                        if method.contains("iter")
                            || method.contains("filter")
                            || method.contains("map") =>
                    {
                        iter_str
                    }
                    _ => format!("{iter_str}.iter().cloned()"),
                };

                let out = if let Some(cond) = condition {
                    let cond_str = self.emit_expr_internal(cond);
                    format!(
                        "{}.filter(|{}| {}).map(|{}| {}).collect::<std::collections::HashSet<_>>()",
                        iter_chain, &target_snake, cond_str, closure_var, elt_str
                    )
                } else {
                    format!(
                        "{iter_chain}.map(|{closure_var}| {elt_str}).collect::<std::collections::HashSet<_>>()"
                    )
                };
                self.shadowed_vars.truncate(old_shadowed_len);
                out
            }
            IrExprKind::Closure {
                params,
                body,
                ret_type,
            } => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| {
                        if p.contains('(') || p.contains(')') || p.contains(',') {
                            p.clone()
                        } else {
                            to_snake_case(p)
                        }
                    })
                    .collect();
                let old_shadowed_len = self.shadowed_vars.len();
                for p in &params_str {
                    self.shadowed_vars.push(p.clone());
                }

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

                let out = format!(
                    "move |{}|{} {{\n{}\n}}",
                    params_str.join(", "),
                    ret_str,
                    body_str
                );
                self.shadowed_vars.truncate(old_shadowed_len);
                out
            }
            IrExprKind::BoxNew(arg) => {
                // Use Arc::new for Callable fields (which are Arc<dyn Fn>)
                format!("std::sync::Arc::new({})", self.emit_expr(arg))
            }
            IrExprKind::Cast { target, ty } => {
                format!("({} as {})", self.emit_expr(target), ty)
            }
            IrExprKind::ConstRef { path } => path.clone(),
            IrExprKind::RawCode(code) => code.clone(),
            IrExprKind::JsonConversion { target, convert_to } => {
                let target_code = self.emit_expr_internal(target);
                // V1.7.0: If target_code ends with '?', it's a Result. 
                // We should wrap it in a parenthesized expression before calling as_xxx().
                let base = if target_code.ends_with('?') {
                    format!("({})", target_code)
                } else {
                    target_code
                };

                match convert_to.as_str() {
                    "f64" => {
                         // TnkValue (serde_json::Value) as_f64 returns Option<f64>
                         format!("{}.as_f64().unwrap()", base)
                    },
                    "i64" => format!("{}.as_i64().unwrap()", base),
                    "String" => format!("{}.as_str().unwrap().to_string()", base),
                    "bool" => format!("{}.as_bool().unwrap()", base),
                    "Vec<f64>" | "Vec<i64>" | "Vec<String>" => {
                        // For vectors, we need more complex conversion if using TnkValue directly
                        // But usually BridgeCall returns TnkValue. 
                        // Let's use a generic from_value if convert_to is complex.
                        self.uses_tsuchinoko_error = true;
                        format!(
                            "serde_json::from_value::<{}>({}).map_err(|e| TsuchinokoError::internal(e.to_string()))?",
                            convert_to, base
                        )
                    }
                    _ => {
                        // V1.7.0: fallback to generic serde_json deserialization
                        self.uses_tsuchinoko_error = true;
                        format!(
                            "serde_json::from_value::<{}>({}).map_err(|e| TsuchinokoError::internal(e.to_string()))?",
                            convert_to, base
                        )
                    }
                }
            }
            IrExprKind::Tuple(elements) => {
                let elems: Vec<_> = elements.iter().map(|e| self.emit_expr(e)).collect();
                format!("({})", elems.join(", "))
            }
            IrExprKind::Index { target, index } => {
                // Handle negative index: arr[-1] -> arr[arr.len()-1]
                // Helper function to extract negative index value
                fn extract_negative_index(expr: &IrExpr) -> Option<i64> {
                    match &expr.kind {
                        // Case 1: UnaryOp { Neg, IntLit(n) }
                        IrExprKind::UnaryOp {
                            op: IrUnaryOp::Neg,
                            operand,
                        } => {
                            if let IrExprKind::IntLit(n) = &operand.kind {
                                return Some(*n);
                            }
                        }
                        // Case 2: IntLit with negative value
                        IrExprKind::IntLit(n) if *n < 0 => {
                            return Some(n.abs());
                        }
                        // Case 3: Cast { target: ..., ty } - unwrap and recurse
                        IrExprKind::Cast { target, .. } => {
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
            IrExprKind::Slice {
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
                    let is_reverse = matches!(&step_expr.kind, IrExprKind::IntLit(-1))
                        || matches!(&step_expr.kind,
                            IrExprKind::UnaryOp { op: IrUnaryOp::Neg, operand }
                            if matches!(&operand.kind, IrExprKind::IntLit(1)));

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
                        (None, Some(e)) => format!("({{ let v = &{target_str}; let e = ({}).min(v.len() as i64).max(0) as usize; v[e..].iter().step_by({step_val} as usize).cloned().collect::<Vec<_>>() }})", self.emit_expr(e)),
                        (Some(s), Some(e)) => format!("({{ let v = &{target_str}; let s = ({}).max(0) as usize; let e = ({}).min(v.len() as i64).max(0) as usize; v[s.min(e)..e].iter().step_by({step_val} as usize).cloned().collect::<Vec<_>>() }})", self.emit_expr(s), self.emit_expr(e)),
                    };
                }

                // Original slice handling (no step)
                match (start, end) {
                    (None, Some(e)) => {
                        // [:n] -> [..(n as usize).min(len)].to_vec()
                        // Handle negative indices: [:-n] -> [..len().saturating_sub(n)].to_vec()
                        if let IrExprKind::UnaryOp {
                            op: IrUnaryOp::Neg,
                            operand,
                        } = &e.kind
                        {
                            if let IrExprKind::IntLit(n) = &operand.kind {
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
                        if let IrExprKind::UnaryOp {
                            op: IrUnaryOp::Neg,
                            operand,
                        } = &s.kind
                        {
                            if let IrExprKind::IntLit(n) = &operand.kind {
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
            IrExprKind::Range { start, end } => {
                format!("{}..{}", self.emit_expr(start), self.emit_expr(end))
            }
            IrExprKind::MethodCall {
                target,
                method,
                args,
                target_type,
                callee_needs_bridge,
            } => {
                let mut args_str: Vec<_> = args.iter().map(|a| self.emit_expr(a)).collect();
                if *callee_needs_bridge {
                    if self.is_inside_resident_func {
                        args_str.insert(0, "py_bridge".to_string());
                    } else {
                        args_str.insert(0, "&mut py_bridge".to_string());
                    }
                }

                let target_str = if let IrExprKind::Var(name) = &target.kind {
                    let var_name = to_snake_case(name);
                    let is_shadowed = self.shadowed_vars.contains(&var_name);
                    let is_try_hoisted = !is_shadowed && self.try_hoisted_vars.contains(&var_name);
                    let is_func_hoisted = !is_shadowed
                        && self
                            .current_hoisted_vars
                            .iter()
                            .any(|v| to_snake_case(&v.name) == var_name);
                    if is_try_hoisted || is_func_hoisted {
                        format!("{}.as_ref().unwrap()", var_name)
                    } else {
                        self.emit_expr_internal(target)
                    }
                } else {
                    self.emit_expr_internal(target)
                };

                if args.is_empty() {
                    if method == "len" {
                        format!("({}.{}() as i64)", target_str, method)
                    } else if method == "copy" {
                        // Python list.copy() -> Rust .to_vec()
                        format!("{}.to_vec()", target_str)
                    } else if method == "collect_hashset" {
                        // V1.5.0: set() constructor -> .collect::<HashSet<_>>()
                        format!(
                            "{}.collect::<std::collections::HashSet<_>>()",
                            target_str
                        )
                    } else if method == "pop" {
                        // V1.5.0: Python list.pop() -> Rust list.pop().unwrap()
                        format!("{}.pop().unwrap()", target_str)
                    } else if method == "clear" {
                        // V1.5.0: Python list.clear() -> Rust list.clear()
                        format!("{}.clear()", target_str)
                    // V1.5.0: String is* methods (no args)
                    } else if method == "isdigit" {
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_ascii_digit())",
                            target_str,
                            target_str
                        )
                    } else if method == "isalpha" {
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_alphabetic())",
                            target_str,
                            target_str
                        )
                    } else if method == "isalnum" {
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_alphanumeric())",
                            target_str,
                            target_str
                        )
                    } else if method == "isupper" {
                        format!(
                            "{}.chars().any(|c| c.is_alphabetic()) && {}.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())",
                            target_str,
                            target_str
                        )
                    } else if method == "islower" {
                        format!(
                            "{}.chars().any(|c| c.is_alphabetic()) && {}.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_lowercase())",
                            target_str,
                            target_str
                        )
                    } else {
                        format!("{}.{}()", target_str, method)
                    }
                } else {
                    // V1.5.0: Set method translations
                    // V1.5.2: Check target_type to correctly handle add method
                    if method == "add" && matches!(target_type, Type::Set(_)) {
                        // Python set.add(x) -> Rust set.insert(x)
                        let args_str: Vec<_> =
                            args.iter().map(|a| self.emit_expr_internal(a)).collect();
                        format!("{}.insert({})", target_str, args_str.join(", "))
                    } else if method == "discard" {
                        // Python set.discard(x) -> Rust set.remove(&x)
                        // Note: set.remove is also &x, but list.remove is handled differently
                        // in semantic analysis (via try_handle_special_method)
                        let arg = &args[0];
                        format!("{}.remove(&{})", target_str, self.emit_expr_internal(arg))
                    } else if method == "pop" && args.len() == 1 {
                        // Python list.pop(i) -> Rust list.remove(i as usize)
                        let idx = &args[0];
                        format!("{}.remove({} as usize)", target_str, self.emit_expr_internal(idx))
                    // Note: list.insert is handled in semantic analysis to distinguish from dict.insert
                    } else if method == "extend" {
                        // Python list.extend(iter) -> Rust list.extend(iter)
                        let iter = &args[0];
                        format!("{}.extend({})", target_str, self.emit_expr_internal(iter))
                    // V1.5.0: String method translations
                    } else if method == "startswith" {
                        // Python s.startswith("x") -> Rust s.starts_with("x")
                        let arg = &args[0];
                        format!("{}.starts_with(&{})", target_str, self.emit_expr_internal(arg))
                    } else if method == "endswith" {
                        // Python s.endswith("x") -> Rust s.ends_with("x")
                        let arg = &args[0];
                        format!("{}.ends_with(&{})", target_str, self.emit_expr_internal(arg))
                    } else if method == "replace" && args.len() >= 2 {
                        // Python s.replace(old, new) -> Rust s.replace(&old, &new)
                        let old = &args[0];
                        let new = &args[1];
                        format!(
                            "{}.replace(&{}, &{})",
                            target_str,
                            self.emit_expr_internal(old),
                            self.emit_expr_internal(new)
                        )
                    } else if method == "find" && args.len() == 1 {
                        // Python s.find(sub) -> Rust s.find(&sub).map(|i| i as i64).unwrap_or(-1)
                        let sub = &args[0];
                        format!(
                            "{}.find(&{}).map(|i| i as i64).unwrap_or(-1i64)",
                            target_str,
                            self.emit_expr_internal(sub)
                        )
                    } else if method == "rfind" && args.len() == 1 {
                        // Python s.rfind(sub) -> Rust s.rfind(&sub).map(|i| i as i64).unwrap_or(-1)
                        let sub = &args[0];
                        format!(
                            "{}.rfind(&{}).map(|i| i as i64).unwrap_or(-1i64)",
                            target_str,
                            self.emit_expr_internal(sub)
                        )
                    } else if method == "isdigit" {
                        // Python s.isdigit() -> Rust s.chars().all(|c| c.is_ascii_digit())
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_ascii_digit())",
                            target_str,
                            target_str
                        )
                    } else if method == "isalpha" {
                        // Python s.isalpha() -> Rust s.chars().all(|c| c.is_alphabetic())
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_alphabetic())",
                            target_str,
                            target_str
                        )
                    } else if method == "isalnum" {
                        // Python s.isalnum() -> Rust s.chars().all(|c| c.is_alphanumeric())
                        format!(
                            "!{}.is_empty() && {}.chars().all(|c| c.is_alphanumeric())",
                            target_str,
                            target_str
                        )
                    } else if method == "isupper" {
                        // Python s.isupper() -> Rust s.chars().any(|c| c.is_alphabetic()) && s.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())
                        format!(
                            "{}.chars().any(|c| c.is_alphabetic()) && {}.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())",
                            target_str,
                            target_str
                        )
                    } else if method == "islower" {
                        // Python s.islower() -> Rust similar logic
                        format!(
                            "{}.chars().any(|c| c.is_alphabetic()) && {}.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_lowercase())",
                            target_str,
                            target_str
                        )
                    } else if method == "count" && args.len() == 1 {
                        // Python s.count(sub) -> Rust s.matches(&sub).count() as i64
                        let sub = &args[0];
                        format!(
                            "{}.matches(&{}).count() as i64",
                            target_str,
                            self.emit_expr_internal(sub)
                        )
                    } else if method == "zfill" && args.len() == 1 {
                        // Python s.zfill(width) -> format!("{:0>width$}", s, width=width)
                        let width = &args[0];
                        format!(
                            "format!(\"{{:0>width$}}\", {}, width = {} as usize)",
                            target_str,
                            self.emit_expr_internal(width)
                        )
                    } else if method == "ljust" && !args.is_empty() {
                        // Python s.ljust(width) -> format!("{:<width$}", s)
                        let width = &args[0];
                        format!(
                            "format!(\"{{:<width$}}\", {}, width = {} as usize)",
                            target_str,
                            self.emit_expr_internal(width)
                        )
                    } else if method == "rjust" && !args.is_empty() {
                        // Python s.rjust(width) -> format!("{:>width$}", s)
                        let width = &args[0];
                        format!(
                            "format!(\"{{:>width$}}\", {}, width = {} as usize)",
                            target_str,
                            self.emit_expr_internal(width)
                        )
                    } else if method == "center" && !args.is_empty() {
                        // Python s.center(width) -> format!("{:^width$}", s)
                        let width = &args[0];
                        format!(
                            "format!(\"{{:^width$}}\", {}, width = {} as usize)",
                            target_str,
                            self.emit_expr_internal(width)
                        )
                    } else {
                        let args_str: Vec<_> =
                            args.iter().map(|a| self.emit_expr_internal(a)).collect();
                        format!(
                            "{}.{}({})",
                            target_str,
                            method,
                            args_str.join(", ")
                        )
                    }
                }
            }
            IrExprKind::PyO3MethodCall {
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
                    "{{\n{}    py_bridge.call_json_method::<TnkValue>({}.clone(), {:?}, &[{}]).map_err(|e| TsuchinokoError::new(\"ExternalError\", &e, None))?\n}}",
                    arg_evals.join("\n    ") + "\n",
                    self.emit_expr_internal(target),
                    method,
                    args_json.join(", ")
                )
            }
            IrExprKind::FieldAccess { target, field } => {
                // Strip dunder prefix for Rust struct field (Python private -> Rust private convention)
                let rust_field = field.trim_start_matches("__");
                format!(
                    "{}.{}",
                    self.emit_expr_internal(target),
                    to_snake_case(rust_field)
                )
            }
            IrExprKind::Reference { target } => {
                if let IrExprKind::Var(name) = &target.kind {
                    let var_name = to_snake_case(name);
                    let is_shadowed = self.shadowed_vars.contains(&var_name);
                    let is_try_hoisted = !is_shadowed && self.try_hoisted_vars.contains(&var_name);
                    let is_func_hoisted = !is_shadowed
                        && self
                            .current_hoisted_vars
                            .iter()
                            .any(|v| to_snake_case(&v.name) == var_name);
                    if is_try_hoisted || is_func_hoisted {
                        format!("{}.as_ref().unwrap()", var_name)
                    } else {
                        format!("&{}", self.emit_expr_internal(target))
                    }
                } else {
                    format!("&{}", self.emit_expr_internal(target))
                }
            }
            IrExprKind::MutReference { target } => {
                if let IrExprKind::Var(name) = &target.kind {
                    let var_name = to_snake_case(name);
                    let is_shadowed = self.shadowed_vars.contains(&var_name);
                    let is_try_hoisted = !is_shadowed && self.try_hoisted_vars.contains(&var_name);
                    let is_func_hoisted = !is_shadowed
                        && self
                            .current_hoisted_vars
                            .iter()
                            .any(|v| to_snake_case(&v.name) == var_name);
                    if is_try_hoisted || is_func_hoisted {
                        format!("{}.as_mut().unwrap()", var_name)
                    } else {
                        format!("&mut {}", self.emit_expr_internal(target))
                    }
                } else {
                    format!("&mut {}", self.emit_expr_internal(target))
                }
            }
            IrExprKind::Print { args } => {
                // Generate println! with type-aware formatting
                // Type::Any uses {} (Display), others use {:?} (Debug)
                if args.is_empty() {
                    "println!()".to_string()
                } else {
                    let format_specs: Vec<&str> = args
                        .iter()
                        .map(|(_, ty)| {
                            if is_display_compatible(ty) {
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
                            if let IrExprKind::StringLit(s) = &expr.kind {
                                format!("\"{s}\"")
                            } else if is_any_type(ty) {
                                // For Type::Any (serde_json::Value), use display_value helper
                                // to handle Value::String without quotes
                                if self.needs_resident {
                                    format!(
                                        "bridge::display_value(&{})",
                                        expr_str.trim_start_matches('&')
                                    )
                                } else {
                                    expr_str
                                }
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
            IrExprKind::Sorted { iter, key, reverse } => {
                let iter_str = self.emit_expr_internal(iter);
                let key_str = key.as_ref().map(|k| self.emit_expr_internal(k));
                let sort_line = if let Some(key_expr) = key_str {
                    format!("v.sort_by_key({key_expr});")
                } else {
                    "v.sort();".to_string()
                };
                let reverse_line = if *reverse { "v.reverse();".to_string() } else { String::new() };
                if reverse_line.is_empty() {
                    format!("{{ let mut v = {}.to_vec(); {} v }}", iter_str, sort_line)
                } else {
                    format!("{{ let mut v = {}.to_vec(); {} {} v }}", iter_str, sort_line, reverse_line)
                }
            }
            // V1.3.1: StructConstruct - semantic now provides field information
            IrExprKind::StructConstruct { name, fields } => {
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
            IrExprKind::DynamicWrap {
                enum_name,
                variant,
                value,
            } => {
                format!("{}::{}({})", enum_name, variant, self.emit_expr(value))
            }
            IrExprKind::Unwrap(inner) => {
                format!("{}.unwrap()", self.emit_expr_internal(inner))
            }
            IrExprKind::BridgeMethodCall {
                target,
                method,
                args,
                keywords,
            } => {
                self.needs_resident = true;
                // V1.5.2: Use ? for error propagation
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;

                let target_str = self.emit_expr(target);
                let args_str: String = args
                    .iter()
                    .map(|a| self.emit_expr(a)) // V1.7.0 Option B: Args are pre-wrapped in Ref/TnkValueFrom
                    .collect::<Vec<_>>()
                    .join(", ");

                // Check if target is BridgeGet (supports fluent syntax)
                let use_method_syntax = match &target.kind {
                    IrExprKind::BridgeGet { .. } => true,
                    IrExprKind::Ref(inner) => matches!(&inner.kind, IrExprKind::BridgeGet { .. }),
                    _ => false,
                };

                if keywords.is_empty() {
                    let call_code = if use_method_syntax {
                        format!("{}.call_method({:?}, &[{}], None)", target_str, method, args_str)
                    } else {
                        format!("py_bridge.call_method(&{}, {:?}, &[{}], None)", target_str, method, args_str)
                    };
                    format!(
                        "{} {}.map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?", 
                        if self.current_func_may_raise { "" } else { "let _ = " }, 
                        call_code
                    )
                } else {
                    let mut kw_setup_code = String::new();
                    let mut kw_inserts = String::new();
                    
                    for (i, (k, v)) in keywords.iter().enumerate() {
                        let val_expr = self.emit_expr(v);
                        kw_setup_code.push_str(&format!("let kw_val_{} = {}; ", i, val_expr));
                        kw_inserts.push_str(&format!("kw.insert({:?}.to_string(), kw_val_{}); ", k, i));
                    }
                    
                    let call_code = if use_method_syntax {
                        format!("{}.call_method({:?}, &[{}], Some(&kw))", target_str, method, args_str)
                    } else {
                        format!("py_bridge.call_method(&{}, {:?}, &[{}], Some(&kw))", target_str, method, args_str)
                    };
                    
                    format!(
                        "{} {{ let mut kw = std::collections::HashMap::new(); {}{} {} }}.map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                        if self.current_func_may_raise { "" } else { "let _ = " },
                        kw_setup_code,
                        kw_inserts,
                        call_code
                    )
                }
            }
            IrExprKind::BridgeCall {
                target,
                args,
                keywords,
            } => {
                self.needs_resident = true;
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;

                let target_str = self.emit_expr(target);
                let args_str: String = args
                    .iter()
                    .map(|a| self.emit_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");

                if keywords.is_empty() {
                    let call_code = format!("{}.call(&[{}], None)", target_str, args_str);
                    format!(
                        "{} {}.map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?", 
                        if self.current_func_may_raise { "" } else { "let _ = " }, 
                        call_code
                    )
                } else {
                    let mut kw_setup_code = String::new();
                    let mut kw_inserts = String::new();
                    for (i, (k, v)) in keywords.iter().enumerate() {
                        let val_expr = self.emit_expr(v);
                        kw_setup_code.push_str(&format!("let kw_val_{} = {}; ", i, val_expr));
                        kw_inserts.push_str(&format!("kw.insert({:?}.to_string(), kw_val_{}); ", k, i));
                    }
                    format!(
                        "({{ let mut kw = std::collections::HashMap::new(); {}{} {}.call(&[{}], Some(&kw)) }}).map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?", 
                        kw_setup_code, kw_inserts, target_str, args_str
                    )
                }
            }
            IrExprKind::Ref(inner) => format!("&{}", self.emit_expr(inner)),
            IrExprKind::TnkValueFrom(inner) => {
                self.emit_as_tnk_value(inner)
            },
            IrExprKind::BridgeAttributeAccess { target, attribute } => {
                self.needs_resident = true;
                self.uses_tsuchinoko_error = true;
                let target_str = self.emit_expr(target);
                
                // Check if target is BridgeGet (supports fluent syntax)
                let use_method_syntax = match &target.kind {
                    IrExprKind::BridgeGet { .. } => true,
                    IrExprKind::Ref(inner) => matches!(&inner.kind, IrExprKind::BridgeGet { .. }),
                    _ => false,
                };
                
                if use_method_syntax {
                     format!(
                        "{}.get_attribute(\"{}\").map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                        target_str, attribute
                    )
                } else {
                    format!(
                        "py_bridge.get_attribute(&{}, \"{}\").map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                        target_str, attribute
                    )
                }
            }
            IrExprKind::BridgeItemAccess { target, index } => {
                self.needs_resident = true;
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;
                let target_str = self.emit_expr(target);
                let index_str = if matches!(&index.kind, IrExprKind::NoneLit) {
                    "TnkValue::Value { value: None }".to_string()
                } else {
                    format!(
                        "TnkValue::from({})",
                        self.emit_expr(index)
                    )
                };

                format!(
                    "py_bridge.get_item(&{}, &{}).map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                    target_str, index_str
                )
            }
            IrExprKind::BridgeSlice {
                target,
                start,
                stop,
                step,
            } => {
                self.needs_resident = true;
                self.current_func_may_raise = true;
                self.uses_tsuchinoko_error = true;
                let target_str = self.emit_expr(target);

                let start_str = if matches!(&start.kind, IrExprKind::NoneLit) {
                    "None".to_string()
                } else {
                    format!(
                        "Some(TnkValue::from({}))",
                        self.emit_expr(start)
                    )
                };
                let stop_str = if matches!(&stop.kind, IrExprKind::NoneLit) {
                    "None".to_string()
                } else {
                    format!(
                        "Some(TnkValue::from({}))",
                        self.emit_expr(stop)
                    )
                };
                let step_str = if matches!(&step.kind, IrExprKind::NoneLit) {
                    "None".to_string()
                } else {
                    format!(
                        "Some(TnkValue::from({}))",
                        self.emit_expr(step)
                    )
                };

                format!(
                    "py_bridge.slice(&{}, {}, {}, {}).map_err(|e| TsuchinokoError::new(\"BridgeError\", &format!(\"{{:?}}\", e), None))?",
                    target_str, start_str, stop_str, step_str
                )
            }
            IrExprKind::BridgeGet { alias } => {
                self.needs_resident = true;
                format!("py_bridge.get(\"{}\")", alias)
            }
            IrExprKind::PyO3Call {
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

                let args_str_list: Vec<String> = (0..args.len()).map(|i| format!("_arg_{i}")).collect();

                // 方式選択テーブルを参照
                use crate::bridge::module_table::{
                    get_import_mode, get_native_binding, ImportMode, NativeBinding,
                };

                match get_import_mode(&target) {
                    ImportMode::Native => {
                        let native_code = match get_native_binding(&target) {
                            Some(NativeBinding::Constant(code)) => code.to_string(),
                            Some(NativeBinding::Method(method)) => {
                                if args_str_list.is_empty() {
                                    format!("/* Native method requires args: {target} */")
                                } else if args_str_list.len() == 1 {
                                    format!("({} as f64).{}()", args_str_list[0], method)
                                } else {
                                    let other_args: Vec<String> = args_str_list
                                        .iter()
                                        .skip(1)
                                        .map(|a| format!("{a} as f64"))
                                        .collect();
                                    format!(
                                        "({} as f64).{}({})",
                                        args_str_list[0],
                                        method,
                                        other_args.join(", ")
                                    )
                                }
                            }
                            None => format!("/* Native not implemented: {target} */"),
                        };
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
                            "{{\n{}    py_bridge.call_json::<TnkValue>(\"{}\", &[{}]).map_err(|e| TsuchinokoError::new(\"ExternalError\", &e, None))?\n}}",
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

fn is_display_compatible(ty: &Type) -> bool {
    match ty {
        Type::Any | Type::String | Type::Int | Type::Float | Type::Bool => true,
        Type::Ref(inner) => is_display_compatible(inner),
        _ => false,
    }
}

fn is_any_type(ty: &Type) -> bool {
    match ty {
        Type::Any => true,
        Type::Ref(inner) => is_any_type(inner),
        _ => false,
    }
}
