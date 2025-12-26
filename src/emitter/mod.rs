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
            IrNode::IndexAssign { target, index, value } => {
                format!("{}{}[({} as usize)] = {};", indent, self.emit_expr(target), self.emit_expr(index), self.emit_expr(value))
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
                if let IrBinOp::Pow = op {
                     return format!("(({} as f64).powf({} as f64) as i64)", self.emit_expr(left), self.emit_expr(right));
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
                
                if func == "print" {
                    // Handle print("msg", arg) -> println!("msg {:?}", arg) or similar
                    // Simplified: just join with spaces, using debug print for non-strings if possible
                    // But format! string is tricky.
                    // For now, let's just emit println!("{:?} {:?}", arg1, arg2)
                    let format_string = std::iter::repeat("{:?}")
                        .take(args.len())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if args.is_empty() {
                         "println!()".to_string()
                    } else {
                        format!("println!(\"{}\", {})", format_string, args_str.join(", "))
                    }
                } else {
                    format!("{}({})", func, args_str.join(", "))
                }
            }
            IrExpr::List { elem_type: _, elements } => {
                let elems: Vec<_> = elements.iter().map(|e| self.emit_expr(e)).collect();
                format!("vec![{}]", elems.join(", "))
            }
            IrExpr::ListComp { elt, target, iter } => {
                // Map strategy: IntoIterator::into_iter(iter).map(|target| elt).collect::<Vec<_>>()
                format!("IntoIterator::into_iter({}).map(|{}| {}).collect::<Vec<_>>()",
                    self.emit_expr(iter),
                    target,
                    self.emit_expr(elt)
                )
            }
            IrExpr::Tuple(elements) => {
                let elems: Vec<_> = elements.iter().map(|e| self.emit_expr(e)).collect();
                format!("({})", elems.join(", "))
            }
            IrExpr::Index { target, index } => {
                format!("{}[({} as usize)]", self.emit_expr(target), self.emit_expr(index))
            }
            IrExpr::Range { start, end } => {
                format!("{}..{}", self.emit_expr(start), self.emit_expr(end))
            }
            IrExpr::MethodCall { target, method, args } => {
                if args.is_empty() {
                    if method == "len" {
                        format!("({}.{}() as i64)", self.emit_expr(target), method)
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
