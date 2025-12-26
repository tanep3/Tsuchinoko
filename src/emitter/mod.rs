//! Emitter module - Rust code generation

use crate::ir::{IrNode, IrExpr, IrBinOp, IrUnaryOp};
use crate::semantic::Type;

/// Emit Rust code from IR
pub fn emit(nodes: &[IrNode]) -> String {
    let mut emitter = RustEmitter::new();
    emitter.emit_nodes(nodes)
}

/// Rust code emitter
pub struct RustEmitter {
    indent: usize,
}

impl RustEmitter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    pub fn emit_nodes(&mut self, nodes: &[IrNode]) -> String {
        nodes
            .iter()
            .map(|n| self.emit_node(n))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn emit_node(&mut self, node: &IrNode) -> String {
        let indent = "    ".repeat(self.indent);
        match node {
            IrNode::VarDecl { name, ty, mutable, init } => {
                let mut_kw = if *mutable { "mut " } else { "" };
                let ty_str = ty.to_rust_string();
                match init {
                    Some(expr) => {
                        format!("{}let {}{}: {} = {};", indent, mut_kw, name, ty_str, self.emit_expr(expr))
                    }
                    None => {
                        format!("{}let {}{}: {};", indent, mut_kw, name, ty_str)
                    }
                }
            }
            IrNode::Assign { target, value } => {
                format!("{}{} = {};", indent, target, self.emit_expr(value))
            }
            IrNode::FuncDecl { name, params, ret, body } => {
                let params_str: Vec<_> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t.to_rust_string()))
                    .collect();
                let ret_str = ret.to_rust_string();
                
                self.indent += 1;
                let body_str = self.emit_nodes(body);
                self.indent -= 1;
                
                format!(
                    "{}fn {}({}) -> {} {{\n{}\n{}}}",
                    indent,
                    name,
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
                    self.emit_expr(cond),
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
                    var,
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
                    self.emit_expr(cond),
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
        }
    }

    fn emit_expr(&self, expr: &IrExpr) -> String {
        match expr {
            IrExpr::IntLit(n) => n.to_string(),
            IrExpr::FloatLit(f) => format!("{:.1}", f),
            IrExpr::StringLit(s) => format!("\"{}\"", s),
            IrExpr::BoolLit(b) => b.to_string(),
            IrExpr::Var(name) => name.clone(),
            IrExpr::BinOp { left, op, right } => {
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
                let args_str: Vec<_> = args.iter().map(|a| self.emit_expr(a)).collect();
                format!("{}({})", func, args_str.join(", "))
            }
            IrExpr::List { elem_type: _, elements } => {
                let elems: Vec<_> = elements.iter().map(|e| self.emit_expr(e)).collect();
                format!("vec![{}]", elems.join(", "))
            }
            IrExpr::Range { start, end } => {
                format!("{}..{}", self.emit_expr(start), self.emit_expr(end))
            }
        }
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
