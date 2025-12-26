//! Semantic analysis module

mod types;
mod scope;

pub use types::*;
pub use scope::*;

use crate::parser::{Program, Stmt, Expr, TypeHint, BinOp as AstBinOp, Param};
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
            Stmt::FuncDef { name, params, return_type, body } => {
                self.scope.push();
                
                // Add parameters to scope
                let ir_params: Vec<(String, Type)> = params
                    .iter()
                    .map(|p| {
                        let ty = p.type_hint.as_ref()
                            .map(|th| self.type_from_hint(th))
                            .unwrap_or(Type::Unknown);
                        self.scope.define(&p.name, ty.clone(), false);
                        (p.name.clone(), ty)
                    })
                    .collect();
                
                let ret_type = return_type.as_ref()
                    .map(|th| self.type_from_hint(th))
                    .unwrap_or(Type::Unit);
                
                let ir_body: Result<Vec<_>, _> = body
                    .iter()
                    .map(|s| self.analyze_stmt(s))
                    .collect();
                
                self.scope.pop();
                
                Ok(IrNode::FuncDecl {
                    name: name.clone(),
                    params: ir_params,
                    ret: ret_type,
                    body: ir_body?,
                })
            }
            Stmt::If { condition, then_body, elif_clauses, else_body } => {
                let ir_cond = self.analyze_expr(condition)?;
                
                self.scope.push();
                let ir_then: Result<Vec<_>, _> = then_body
                    .iter()
                    .map(|s| self.analyze_stmt(s))
                    .collect();
                self.scope.pop();
                
                // Combine elif clauses into nested if-else
                let mut ir_else: Option<Vec<IrNode>> = if let Some(else_stmts) = else_body {
                    self.scope.push();
                    let result: Result<Vec<_>, _> = else_stmts
                        .iter()
                        .map(|s| self.analyze_stmt(s))
                        .collect();
                    self.scope.pop();
                    Some(result?)
                } else {
                    None
                };
                
                // Process elif clauses in reverse to nest them
                for (elif_cond, elif_body) in elif_clauses.iter().rev() {
                    let elif_ir_cond = self.analyze_expr(elif_cond)?;
                    self.scope.push();
                    let elif_ir_body: Result<Vec<_>, _> = elif_body
                        .iter()
                        .map(|s| self.analyze_stmt(s))
                        .collect();
                    self.scope.pop();
                    
                    let nested_if = IrNode::If {
                        cond: Box::new(elif_ir_cond),
                        then_block: elif_ir_body?,
                        else_block: ir_else,
                    };
                    ir_else = Some(vec![nested_if]);
                }
                
                Ok(IrNode::If {
                    cond: Box::new(ir_cond),
                    then_block: ir_then?,
                    else_block: ir_else,
                })
            }
            Stmt::For { target, iter, body } => {
                let ir_iter = self.analyze_expr(iter)?;
                
                self.scope.push();
                self.scope.define(target, Type::Int, false); // Assume int for now
                
                let ir_body: Result<Vec<_>, _> = body
                    .iter()
                    .map(|s| self.analyze_stmt(s))
                    .collect();
                self.scope.pop();
                
                Ok(IrNode::For {
                    var: target.clone(),
                    var_type: Type::Int,
                    iter: Box::new(ir_iter),
                    body: ir_body?,
                })
            }
            Stmt::While { condition, body } => {
                let ir_cond = self.analyze_expr(condition)?;
                
                self.scope.push();
                let ir_body: Result<Vec<_>, _> = body
                    .iter()
                    .map(|s| self.analyze_stmt(s))
                    .collect();
                self.scope.pop();
                
                Ok(IrNode::While {
                    cond: Box::new(ir_cond),
                    body: ir_body?,
                })
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
        }
    }

    fn analyze_expr(&self, expr: &Expr) -> Result<IrExpr, TsuchinokoError> {
        match expr {
            Expr::IntLiteral(n) => Ok(IrExpr::IntLit(*n)),
            Expr::FloatLiteral(f) => Ok(IrExpr::FloatLit(*f)),
            Expr::StringLiteral(s) => Ok(IrExpr::StringLit(s.clone())),
            Expr::BoolLiteral(b) => Ok(IrExpr::BoolLit(*b)),
            Expr::NoneLiteral => Ok(IrExpr::IntLit(0)),
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
    fn test_analyze_function_def() {
        let code = r#"
def add(a: int, b: int) -> int:
    return a + b
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert_eq!(ir.len(), 1);
        
        if let IrNode::FuncDecl { name, params, ret, body } = &ir[0] {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(*ret, Type::Int);
            assert_eq!(body.len(), 1);
        }
    }

    #[test]
    fn test_analyze_if_stmt() {
        let code = r#"
if x > 0:
    y = 1
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert_eq!(ir.len(), 1);
        
        if let IrNode::If { then_block, .. } = &ir[0] {
            assert_eq!(then_block.len(), 1);
        }
    }

    #[test]
    fn test_analyze_for_loop() {
        let code = r#"
for i in range(10):
    x = i
"#;
        let program = parse(code).unwrap();
        let ir = analyze(&program).unwrap();
        assert_eq!(ir.len(), 1);
        
        if let IrNode::For { var, body, .. } = &ir[0] {
            assert_eq!(var, "i");
            assert_eq!(body.len(), 1);
        }
    }
}
