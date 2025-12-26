//! Semantic analysis module

mod types;
mod scope;

pub use types::*;
pub use scope::*;

use crate::parser::{Program, Stmt, Expr, TypeHint, BinOp as AstBinOp};
use crate::ir::{IrNode, IrExpr, IrBinOp};
use crate::error::TsuchinokoError;

/// Analyze a program and convert to IR
pub fn analyze(program: &Program) -> Result<Vec<IrNode>, TsuchinokoError> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program)
}

/// Semantic analyzer
pub struct SemanticAnalyzer {
    scope: ScopeStack,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            scope: ScopeStack::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<Vec<IrNode>, TsuchinokoError> {
        let mut ir_nodes = Vec::new();
        
        for stmt in &program.statements {
            ir_nodes.push(self.analyze_stmt(stmt)?);
        }
        
        Ok(ir_nodes)
    }

    fn analyze_stmt(&mut self, stmt: &Stmt) -> Result<IrNode, TsuchinokoError> {
        match stmt {
            Stmt::Assign { target, type_hint, value } => {
                let ty = match type_hint {
                    Some(th) => self.type_from_hint(th),
                    None => self.infer_type(value),
                };
                
                // Check if variable already exists (mutable)
                let mutable = self.scope.lookup(target).is_some();
                
                if !mutable {
                    self.scope.define(target, ty.clone(), false);
                }
                
                let ir_value = self.analyze_expr(value)?;
                
                if mutable {
                    Ok(IrNode::Assign {
                        target: target.clone(),
                        value: Box::new(ir_value),
                    })
                } else {
                    Ok(IrNode::VarDecl {
                        name: target.clone(),
                        ty,
                        mutable: false,
                        init: Some(Box::new(ir_value)),
                    })
                }
            }
            Stmt::Return(expr) => {
                let ir_expr = match expr {
                    Some(e) => Some(Box::new(self.analyze_expr(e)?)),
                    None => None,
                };
                Ok(IrNode::Return(ir_expr))
            }
            Stmt::Expr(expr) => {
                let ir_expr = self.analyze_expr(expr)?;
                Ok(IrNode::Expr(ir_expr))
            }
            _ => {
                // TODO: Handle other statement types
                Ok(IrNode::Expr(IrExpr::IntLit(0)))
            }
        }
    }

    fn analyze_expr(&self, expr: &Expr) -> Result<IrExpr, TsuchinokoError> {
        match expr {
            Expr::IntLiteral(n) => Ok(IrExpr::IntLit(*n)),
            Expr::FloatLiteral(f) => Ok(IrExpr::FloatLit(*f)),
            Expr::StringLiteral(s) => Ok(IrExpr::StringLit(s.clone())),
            Expr::BoolLiteral(b) => Ok(IrExpr::BoolLit(*b)),
            Expr::NoneLiteral => Ok(IrExpr::IntLit(0)), // TODO: Handle None properly
            Expr::Ident(name) => Ok(IrExpr::Var(name.clone())),
            Expr::BinOp { left, op, right } => {
                let ir_left = self.analyze_expr(left)?;
                let ir_right = self.analyze_expr(right)?;
                let ir_op = self.convert_binop(op);
                Ok(IrExpr::BinOp {
                    left: Box::new(ir_left),
                    op: ir_op,
                    right: Box::new(ir_right),
                })
            }
            Expr::Call { func, args } => {
                // Special handling for range()
                if func == "range" {
                    if args.len() == 1 {
                        let end = self.analyze_expr(&args[0])?;
                        return Ok(IrExpr::Range {
                            start: Box::new(IrExpr::IntLit(0)),
                            end: Box::new(end),
                        });
                    } else if args.len() == 2 {
                        let start = self.analyze_expr(&args[0])?;
                        let end = self.analyze_expr(&args[1])?;
                        return Ok(IrExpr::Range {
                            start: Box::new(start),
                            end: Box::new(end),
                        });
                    }
                }
                
                let ir_args: Result<Vec<_>, _> = args
                    .iter()
                    .map(|a| self.analyze_expr(a))
                    .collect();
                Ok(IrExpr::Call {
                    func: func.clone(),
                    args: ir_args?,
                })
            }
            Expr::List(elements) => {
                let ir_elements: Result<Vec<_>, _> = elements
                    .iter()
                    .map(|e| self.analyze_expr(e))
                    .collect();
                let elem_type = if let Some(first) = elements.first() {
                    self.infer_type(first)
                } else {
                    Type::Unknown
                };
                Ok(IrExpr::List {
                    elem_type,
                    elements: ir_elements?,
                })
            }
            _ => Ok(IrExpr::IntLit(0)),
        }
    }

    fn type_from_hint(&self, hint: &TypeHint) -> Type {
        let params: Vec<Type> = hint.params.iter().map(|h| self.type_from_hint(h)).collect();
        Type::from_python_hint(&hint.name, &params)
    }

    fn infer_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::IntLiteral(_) => Type::Int,
            Expr::FloatLiteral(_) => Type::Float,
            Expr::StringLiteral(_) => Type::String,
            Expr::BoolLiteral(_) => Type::Bool,
            Expr::List(elements) => {
                if let Some(first) = elements.first() {
                    Type::List(Box::new(self.infer_type(first)))
                } else {
                    Type::List(Box::new(Type::Unknown))
                }
            }
            Expr::Ident(name) => {
                if let Some(info) = self.scope.lookup(name) {
                    info.ty.clone()
                } else {
                    Type::Unknown
                }
            }
            _ => Type::Unknown,
        }
    }

    fn convert_binop(&self, op: &AstBinOp) -> IrBinOp {
        match op {
            AstBinOp::Add => IrBinOp::Add,
            AstBinOp::Sub => IrBinOp::Sub,
            AstBinOp::Mul => IrBinOp::Mul,
            AstBinOp::Div => IrBinOp::Div,
            AstBinOp::Mod => IrBinOp::Mod,
            AstBinOp::Eq => IrBinOp::Eq,
            AstBinOp::NotEq => IrBinOp::NotEq,
            AstBinOp::Lt => IrBinOp::Lt,
            AstBinOp::Gt => IrBinOp::Gt,
            AstBinOp::LtEq => IrBinOp::LtEq,
            AstBinOp::GtEq => IrBinOp::GtEq,
            AstBinOp::And => IrBinOp::And,
            AstBinOp::Or => IrBinOp::Or,
            _ => IrBinOp::Add,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_analyze_simple_assignment() {
        let program = parse("x: int = 10").unwrap();
        let ir = analyze(&program).unwrap();
        assert_eq!(ir.len(), 1);
        
        if let IrNode::VarDecl { name, ty, .. } = &ir[0] {
            assert_eq!(name, "x");
            assert_eq!(*ty, Type::Int);
        } else {
            panic!("Expected VarDecl");
        }
    }

    #[test]
    fn test_analyze_binary_op() {
        let program = parse("result: int = a + b").unwrap();
        let ir = analyze(&program).unwrap();
        
        if let IrNode::VarDecl { init: Some(expr), .. } = &ir[0] {
            if let IrExpr::BinOp { op, .. } = expr.as_ref() {
                assert_eq!(*op, IrBinOp::Add);
            }
        }
    }
}
