//! Emitter module - Rust code generation

use crate::ir::{IrNode, IrExpr, IrBinOp, IrUnaryOp};
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
    fn emit_expr(&self, expr: &IrExpr) -> String;
    
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

impl RustEmitter {
    pub fn new() -> Self {
        Self { indent: 0, struct_defs: HashMap::new() }
    }

    pub fn emit_nodes(&mut self, nodes: &[IrNode]) -> String {
        nodes
            .iter()
            .map(|n| self.emit_node(n))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn emit_node_internal(&mut self, node: &IrNode) -> String {
        let indent = "    ".repeat(self.indent);
        match node {
            IrNode::VarDecl { name, ty, mutable, init } => {
                let mut_kw = if *mutable { "mut " } else { "" };
                let ty_str = ty.to_rust_string();
                let snake_name = to_snake_case(name);
                match init {
                    Some(expr) => {
                        format!("{}let {}{}: {} = {};", indent, mut_kw, snake_name, ty_str, self.emit_expr_no_outer_parens(expr))
                    }
                    None => {
                        format!("{}let {}{}: {};", indent, mut_kw, snake_name, ty_str)
                    }
                }
            }
            IrNode::Assign { target, value } => {
                format!("{}{} = {};", indent, to_snake_case(target), self.emit_expr(value))
            }
            IrNode::IndexAssign { target, index, value } => {
                format!("{}{}[{} as usize] = {};", indent, self.emit_expr(target), self.emit_expr(index), self.emit_expr(value))
            }
            IrNode::MultiAssign { targets, value } => {
                let targets_str = targets.join(", ");
                format!("{}({}) = {};", indent, targets_str, self.emit_expr(value))
            }
            IrNode::MultiVarDecl { targets, value } => {
                let vars_str: Vec<_> = targets.iter()
                    .map(|(n, _, m)| {
                        let mut_kw = if *m { "mut " } else { "" };
                        format!("{}{}", mut_kw, n)
                    })
                    .collect();
                
                // We typically don't need types in the pattern if they are inferred from the right side,
                // but since we resolved them, we could add type annotation to the let binding if we wanted.
                // However, syntax "let (x, y): (int, int) = ..." works.
                // For simplicity, let's try to trust inference or just emit the pattern "let (mut x, y) = ..."
                // Adding full type annotation for tuple destructuring "let (x, y): (T1, T2)" is also good.
                
                let types_str: Vec<_> = targets.iter()
                    .map(|(_, t, _)| t.to_rust_string())
                    .collect();
                
                format!("{}let ({}) : ({}) = {};", 
                    indent, 
                    vars_str.join(", "), 
                    types_str.join(", "), 
                    self.emit_expr(value)
                )
            }
            IrNode::FuncDecl { name, params, ret, body } => {
                let snake_name = if name == "main" { name.clone() } else { to_snake_case(name) };
                let params_str: Vec<_> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", to_snake_case(n), t.to_rust_string()))
                    .collect();
                let ret_str = ret.to_rust_string();
                
                self.indent += 1;
                let body_str = self.emit_nodes(body);
                self.indent -= 1;
                
                format!(
                    "{}fn {}({}) -> {} {{\n{}\n{}}}",
                    indent,
                    snake_name,
                    params_str.join(", "),
                    ret_str,
                    body_str,
                    indent
                )
            }
            IrNode::If { cond, then_block, else_block } => {
                self.indent += 1;
                let then_str = self.emit_nodes(then_block);
                self.indent -= 1;
                
                let else_str = match else_block {
                    Some(block) => {
                        self.indent += 1;
                        let s = self.emit_nodes(block);
                        self.indent -= 1;
                        format!(" else {{\n{}\n{}}}", s, indent)
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
            IrNode::For { var, var_type: _, iter, body } => {
                self.indent += 1;
                let body_str = self.emit_nodes(body);
                self.indent -= 1;
                
                format!(
                    "{}for {} in {} {{\n{}\n{}}}",
                    indent,
                    to_snake_case(var),
                    self.emit_expr(iter),
                    body_str,
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
                match expr {
                    Some(e) => format!("{}return {};", indent, self.emit_expr(e)),
                    None => format!("{}return;", indent),
                }
            }
            IrNode::Expr(expr) => {
                format!("{}{};", indent, self.emit_expr(expr))
            }
            IrNode::StructDef { name, fields } => {
                // Register struct definition for constructor emission
                let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                self.struct_defs.insert(name.clone(), field_names);
                
                let mut result = format!("{}#[derive(Clone, Debug)]\n", indent);
                result.push_str(&format!("{}struct {} {{\n", indent, name));
                for (field_name, field_type) in fields {
                    let rust_type = field_type.to_rust_string();
                    result.push_str(&format!("{}    {}: {},\n", indent, to_snake_case(field_name), rust_type));
                }
                result.push_str(&format!("{}}}", indent));
                result
            }
            IrNode::TryBlock { try_body, except_body } => {
                // For now, emit as a comment explaining the limitation
                // In a full implementation, we would need to analyze the try body
                // to determine what operations return Result and wrap them accordingly
                let mut result = format!("{}// try-except: simplified translation\n", indent);
                result.push_str(&format!("{}// Try block:\n", indent));
                for node in try_body {
                    result.push_str(&self.emit_node(node));
                    result.push('\n');
                }
                result.push_str(&format!("{}// Except block (fallback - not automatically invoked):\n", indent));
                for node in except_body {
                    result.push_str(&format!("{}// {}\n", indent, self.emit_node(node).trim()));
                }
                result
            }
        }
    }

    fn emit_expr_internal(&self, expr: &IrExpr) -> String {
        match expr {
            IrExpr::IntLit(n) => n.to_string(),
            IrExpr::FloatLit(f) => format!("{:.1}", f),
            IrExpr::StringLit(s) => format!("\"{}\"", s),
            IrExpr::BoolLit(b) => b.to_string(),
            IrExpr::NoneLit => "None".to_string(),
            IrExpr::Var(name) => to_snake_case(name),
            IrExpr::BinOp { left, op, right } => {
                if let IrBinOp::Pow = op {
                     return format!("({} as i64).pow(({}) as u32)", self.emit_expr(left), self.emit_expr(right));
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
                    IrBinOp::Contains => {
                        // x in dict -> dict.contains_key(&x)
                        return format!("{}.contains_key(&{})", self.emit_expr(right), self.emit_expr(left));
                    }
                };
                format!("({} {} {})", self.emit_expr(left), op_str, self.emit_expr(right))
            }
            IrExpr::UnaryOp { op, operand } => {
                let op_str = match op {
                    IrUnaryOp::Neg => "-",
                    IrUnaryOp::Not => "!",
                };
                format!("({}{})", op_str, self.emit_expr(operand))
            }
            IrExpr::Call { func, args } => {
                if func == "print" {
                    // Handle print("msg", arg) -> println!("msg {:?}", arg)
                    // Clean up: remove .to_string() for string literals and .clone() for println
                    let cleaned_args: Vec<_> = args.iter().map(|a| {
                        // Unwrap unnecessary MethodCall wrappers
                        let unwrapped = match a {
                            IrExpr::MethodCall { target, method, args: mc_args } 
                                if mc_args.is_empty() && (method == "clone" || method == "to_string") => 
                            {
                                target.as_ref()
                            }
                            other => other,
                        };
                        
                        // For string literals, emit directly
                        match unwrapped {
                            IrExpr::StringLit(s) => format!("\"{}\"", s),
                            _ => {
                                // Just pass by ref for println
                                let expr_str = self.emit_expr(unwrapped);
                                if expr_str.starts_with('&') {
                                    expr_str
                                } else {
                                    format!("&{}", expr_str)
                                }
                            }
                        }
                    }).collect();
                    
                    let format_string = std::iter::repeat("{:?}")
                        .take(args.len())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if args.is_empty() {
                         "println!()".to_string()
                    } else {
                        format!("println!(\"{}\", {})", format_string, cleaned_args.join(", "))
                    }
                } else {
                    // Check if this is a struct constructor
                    if let Some(field_names) = self.struct_defs.get(func) {
                        // Emit as struct literal: Point { x: 0, y: 0 }
                        let args_str: Vec<_> = args.iter().map(|a| self.emit_expr_no_outer_parens(a)).collect();
                        let field_inits: Vec<String> = field_names.iter().zip(args_str.iter())
                            .map(|(name, value)| format!("{}: {}", to_snake_case(name), value))
                            .collect();
                        format!("{} {{ {} }}", func, field_inits.join(", "))
                    } else {
                        let args_str: Vec<_> = args.iter().map(|a| self.emit_expr_no_outer_parens(a)).collect();
                        // Don't snake_case built-in Rust expressions like Some, None, Ok, Err
                        let func_name = if func == "Some" || func == "None" || func == "Ok" || func == "Err" {
                            func.clone()
                        } else {
                            to_snake_case(func)
                        };
                        format!("{}({})", func_name, args_str.join(", "))
                    }
                }
            }
            IrExpr::List { elem_type: _, elements } => {
                let elems: Vec<_> = elements.iter().map(|e| self.emit_expr(e)).collect();
                format!("vec![{}]", elems.join(", "))
            }
            IrExpr::Dict { key_type: _, value_type: _, entries } => {
                if entries.is_empty() {
                    "std::collections::HashMap::new()".to_string()
                } else {
                    let pairs: Vec<_> = entries.iter()
                        .map(|(k, v)| {
                            // For string keys, add .to_string()
                            let key_str = match k {
                                IrExpr::StringLit(s) => format!("\"{}\".to_string()", s),
                                _ => self.emit_expr(k),
                            };
                            format!("({}, {})", key_str, self.emit_expr(v))
                        })
                        .collect();
                    format!("std::collections::HashMap::from([{}])", pairs.join(", "))
                }
            }
            IrExpr::ListComp { elt, target, iter } => {
                // Map strategy: IntoIterator::into_iter(iter).map(|target| elt).collect::<Vec<_>>()
                // If elt doesn't reference target, use _ to avoid unused variable warning
                let elt_str = self.emit_expr(elt);
                let target_snake = to_snake_case(target);
                let closure_var = if elt_str.contains(&target_snake) {
                    target_snake
                } else {
                    "_".to_string()
                };
                format!("IntoIterator::into_iter({}).map(|{}| {}).collect::<Vec<_>>()",
                    self.emit_expr(iter),
                    closure_var,
                    elt_str
                )
            }
            IrExpr::Tuple(elements) => {
                let elems: Vec<_> = elements.iter().map(|e| self.emit_expr(e)).collect();
                format!("({})", elems.join(", "))
            }
            IrExpr::Index { target, index } => {
                // Handle negative index: arr[-1] -> arr[arr.len()-1]
                // Case 1: UnaryOp { Neg, IntLit(n) }
                if let IrExpr::UnaryOp { op: IrUnaryOp::Neg, operand } = index.as_ref() {
                    if let IrExpr::IntLit(n) = operand.as_ref() {
                        let target_str = self.emit_expr(target);
                        return format!("{}[{}.len() - {}]", target_str, target_str, n);
                    }
                }
                // Case 2: IntLit with negative value
                if let IrExpr::IntLit(n) = index.as_ref() {
                    if *n < 0 {
                        let target_str = self.emit_expr(target);
                        return format!("{}[{}.len() - {}]", target_str, target_str, n.abs());
                    }
                }
                format!("{}[{} as usize]", self.emit_expr(target), self.emit_expr(index))
            }
            IrExpr::Range { start, end } => {
                // If both start and end are integer literals, emit as usize range
                // This allows for i in 0..10 to produce usize loop variable
                let start_lit = matches!(start.as_ref(), IrExpr::IntLit(_));
                let end_lit = matches!(end.as_ref(), IrExpr::IntLit(_));
                if start_lit && end_lit {
                    format!("{}usize..{}usize", self.emit_expr(start), self.emit_expr(end))
                } else {
                    format!("{}..{}", self.emit_expr(start), self.emit_expr(end))
                }
            }
            IrExpr::MethodCall { target, method, args } => {
                if args.is_empty() {
                    if method == "len" {
                        format!("{}.{}() as i64", self.emit_expr(target), method)
                    } else {
                        format!("{}.{}()", self.emit_expr(target), method)
                    }
                } else {
                    let args_str: Vec<_> = args.iter().map(|a| self.emit_expr(a)).collect();
                    format!("{}.{}({})", self.emit_expr(target), method, args_str.join(", "))
                }
            }
            IrExpr::FieldAccess { target, field } => {
                format!("{}.{}", self.emit_expr(target), field)
            }
            IrExpr::Reference { target } => {
                format!("&{}", self.emit_expr(target))
            }
        }
    }

    /// Emit expression without outer parentheses (for if/while conditions)
    fn emit_expr_no_outer_parens(&self, expr: &IrExpr) -> String {
        let s = self.emit_expr(expr);
        if s.starts_with('(') && s.ends_with(')') {
            // Check if these are matching outer parens
            let inner = &s[1..s.len()-1];
            // Simple check: if inner has balanced parens, strip outer
            let mut depth = 0;
            let mut valid = true;
            for c in inner.chars() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth < 0 { valid = false; break; }
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
    
    fn emit_expr(&self, expr: &IrExpr) -> String {
        // Delegate to the internal implementation  
        RustEmitter::emit_expr_internal(self, expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_var_decl() {
        let node = IrNode::VarDecl {
            name: "x".to_string(),
            ty: Type::Int,
            mutable: false,
            init: Some(Box::new(IrExpr::IntLit(42))),
        };
        let result = emit(&[node]);
        assert_eq!(result, "let x: i64 = 42;");
    }

    #[test]
    fn test_emit_function() {
        let node = IrNode::FuncDecl {
            name: "add".to_string(),
            params: vec![
                ("a".to_string(), Type::Int),
                ("b".to_string(), Type::Int),
            ],
            ret: Type::Int,
            body: vec![
                IrNode::Return(Some(Box::new(IrExpr::BinOp {
                    left: Box::new(IrExpr::Var("a".to_string())),
                    op: IrBinOp::Add,
                    right: Box::new(IrExpr::Var("b".to_string())),
                }))),
            ],
        };
        let result = emit(&[node]);
        assert!(result.contains("fn add(a: i64, b: i64) -> i64"));
        assert!(result.contains("return (a + b)"));
    }
}
