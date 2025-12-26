//! Semantic analysis module

mod types;
mod scope;

pub use types::*;
pub use scope::*;

use crate::parser::{Program, Stmt, Expr, TypeHint, BinOp as AstBinOp, UnaryOp as AstUnaryOp, Param};
use crate::ir::{IrNode, IrExpr, IrBinOp, IrUnaryOp};
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
            // Check for if __name__ == "__main__" pattern
            if let Stmt::If { condition, then_body, elif_clauses, else_body } = stmt {
                if elif_clauses.is_empty() && else_body.is_none() {
                    if let Expr::BinOp { left, op, right } = condition {
                        if let (Expr::Ident(l), AstBinOp::Eq, Expr::StringLiteral(r)) = (left.as_ref(), op, right.as_ref()) {
                            if l == "__name__" && r == "__main__" {
                                // Convert to fn main()
                                self.scope.push();
                                let mut body_nodes = Vec::new();
                                for s in then_body {
                                    body_nodes.push(self.analyze_stmt(s)?);
                                }
                                self.scope.pop();
                                
                                ir_nodes.push(IrNode::FuncDecl {
                                    name: "main".to_string(),
                                    params: vec![],
                                    ret: Type::Unit,
                                    body: body_nodes,
                                });
                                continue;
                            }
                        }
                    }
                }
            }
            
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
                
                // Check if variable is already defined (re-assignment)
                let is_reassign = self.scope.lookup(target).is_some();
                
                // In Python, lists are always mutable. In Rust, we should make them mutable by default
                // to allow modification (like push, index assign).
                // Also respect re-assignment.
                let should_be_mutable = is_reassign || matches!(ty, Type::List(_));
                
                if !is_reassign {
                    self.scope.define(target, ty.clone(), false);
                }
                
                let ir_value = self.analyze_expr(value)?;
                
                if is_reassign {
                    Ok(IrNode::Assign {
                        target: target.clone(),
                        value: Box::new(ir_value),
                    })
                } else {
                    Ok(IrNode::VarDecl {
                        name: target.clone(),
                        ty,
                        mutable: should_be_mutable,
                        init: Some(Box::new(ir_value)),
                    })
                }
            }
            Stmt::IndexAssign { target, index, value } => {
                let ir_target = self.analyze_expr(target)?;
                let ir_index = self.analyze_expr(index)?;
                let ir_value = self.analyze_expr(value)?;
                Ok(IrNode::IndexAssign {
                    target: Box::new(ir_target),
                    index: Box::new(ir_index),
                    value: Box::new(ir_value),
                })
            }
            Stmt::TupleAssign { targets, value } => {
                // Determine if this is a declaration or assignment based on first variable
                // (Simplified logic: if first var is not in scope, assume declaration for all)
                let is_decl = self.scope.lookup(&targets[0]).is_none();
                let ir_value = self.analyze_expr(value)?;
                
                if is_decl {
                    // Try to infer types if possible, otherwise Unknown
                    // If value is a call, we might not know the return type yet without a better symbol table
                    let result_type = self.infer_type(value);
                    let elem_types = if let Type::Tuple(types) = result_type {
                        if types.len() == targets.len() {
                            types
                        } else {
                            vec![Type::Unknown; targets.len()]
                        }
                    } else {
                        vec![Type::Unknown; targets.len()]
                    };
                    
                    let mut decl_targets = Vec::new();
                    for (i, target) in targets.iter().enumerate() {
                        let ty = elem_types.get(i).unwrap_or(&Type::Unknown).clone();
                        self.scope.define(target, ty.clone(), false);
                        decl_targets.push((target.clone(), ty, false));
                    }
                    
                    Ok(IrNode::MultiVarDecl {
                        targets: decl_targets,
                        value: Box::new(ir_value),
                    })
                } else {
                    // Assignment to existing variables
                    Ok(IrNode::MultiAssign {
                        targets: targets.clone(),
                        value: Box::new(ir_value),
                    })
                }
            }
            Stmt::FuncDef { name, params, return_type, body } => {
                 self.scope.push();
                 
                 // Add parameters to scope
                 let mut ir_params = Vec::new();
                 for p in params {
                     let ty = p.type_hint.as_ref()
                         .map(|th| self.type_from_hint(th))
                         .unwrap_or(Type::Unknown);
                     self.scope.define(&p.name, ty.clone(), false);
                     ir_params.push((p.name.clone(), ty));
                 }
                 
                 let ret_type = return_type.as_ref()
                     .map(|th| self.type_from_hint(th))
                     .unwrap_or(Type::Unit);
                 
                 let mut ir_body = Vec::new();
                 for s in body {
                     ir_body.push(self.analyze_stmt(s)?);
                 }
                 
                 self.scope.pop();
                 
                 let ir_name = if name == "main" { "user_main".to_string() } else { name.clone() };
                 Ok(IrNode::FuncDecl {
                     name: ir_name,
                     params: ir_params,
                     ret: ret_type,
                     body: ir_body,
                 })
            }
            Stmt::If { condition, then_body, elif_clauses, else_body } => {
                // Check for main block
                if let Expr::BinOp { left, op, right } = condition {
                    if let (Expr::Ident(l), AstBinOp::Eq, Expr::StringLiteral(r)) = (left.as_ref(), op, right.as_ref()) {
                        if l == "__name__" && r == "__main__" {
                             self.scope.push();
                             let mut ir_body = Vec::new();
                             for s in then_body {
                                 ir_body.push(self.analyze_stmt(s)?);
                             }
                             self.scope.pop();
                             
                             return Ok(IrNode::FuncDecl {
                                 name: "main".to_string(),
                                 params: vec![],
                                 ret: Type::Unit,
                                 body: ir_body,
                             });
                        }
                    }
                }

                let ir_cond = self.analyze_expr(condition)?;
                
                self.scope.push();
                let mut ir_then = Vec::new();
                for s in then_body {
                    ir_then.push(self.analyze_stmt(s)?);
                }
                self.scope.pop();
                
                let mut ir_else = if let Some(else_stmts) = else_body {
                    self.scope.push();
                    let mut stmts = Vec::new();
                    for s in else_stmts {
                        stmts.push(self.analyze_stmt(s)?);
                    }
                    self.scope.pop();
                    Some(stmts)
                } else {
                    None
                };
                
                for (elif_cond, elif_body) in elif_clauses.iter().rev() {
                    let elif_ir_cond = self.analyze_expr(elif_cond)?;
                    self.scope.push();
                    let mut elif_ir_body = Vec::new();
                    for s in elif_body {
                        elif_ir_body.push(self.analyze_stmt(s)?);
                    }
                    self.scope.pop();
                    
                    let nested_if = IrNode::If {
                        cond: Box::new(elif_ir_cond),
                        then_block: elif_ir_body,
                        else_block: ir_else,
                    };
                    ir_else = Some(vec![nested_if]);
                }
                
                Ok(IrNode::If {
                    cond: Box::new(ir_cond),
                    then_block: ir_then,
                    else_block: ir_else,
                })
            }
            Stmt::For { target, iter, body } => {
                let ir_iter = self.analyze_expr(iter)?;
                
                self.scope.push();
                self.scope.define(target, Type::Int, false);
                
                let mut ir_body = Vec::new();
                for s in body {
                    ir_body.push(self.analyze_stmt(s)?);
                }
                self.scope.pop();
                
                Ok(IrNode::For {
                    var: target.clone(),
                    var_type: Type::Int,
                    iter: Box::new(ir_iter),
                    body: ir_body,
                })
            }
            Stmt::While { condition, body } => {
                let ir_cond = self.analyze_expr(condition)?;
                
                self.scope.push();
                let mut ir_body = Vec::new();
                for s in body {
                    ir_body.push(self.analyze_stmt(s)?);
                }
                self.scope.pop();
                
                Ok(IrNode::While {
                    cond: Box::new(ir_cond),
                    body: ir_body,
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

    fn analyze_expr(&mut self, expr: &Expr) -> Result<IrExpr, TsuchinokoError> {
        match expr {
            Expr::IntLiteral(n) => Ok(IrExpr::IntLit(*n)),
            Expr::FloatLiteral(f) => Ok(IrExpr::FloatLit(*f)),
            Expr::StringLiteral(s) => Ok(IrExpr::StringLit(s.clone())),
            Expr::BoolLiteral(b) => Ok(IrExpr::BoolLit(*b)),
            Expr::NoneLiteral => Ok(IrExpr::IntLit(0)), // Python None -> Rust 0 (temporary hack)
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
                 match func.as_ref() {
                    Expr::Ident(name) => {
                         // Range handling
                        if name == "range" {
                            if args.len() == 1 {
                                let start = IrExpr::IntLit(0);
                                let end = self.analyze_expr(&args[0])?;
                                return Ok(IrExpr::Range {
                                    start: Box::new(start),
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
                        
                         // Built-in functions
                        if name == "len" && args.len() == 1 {
                            let arg = self.analyze_expr(&args[0])?;
                            return Ok(IrExpr::MethodCall { target: Box::new(arg), method: "len".to_string(), args: vec![] });
                        }
                        if name == "list" && args.len() == 1 {
                            let arg = self.analyze_expr(&args[0])?;
                            return Ok(IrExpr::MethodCall { target: Box::new(arg), method: "clone".to_string(), args: vec![] });
                        }
                        if name == "str" && args.len() == 1 {
                            let arg = self.analyze_expr(&args[0])?;
                            return Ok(IrExpr::MethodCall { target: Box::new(arg), method: "to_string".to_string(), args: vec![] });
                        }
                        if name == "max" && args.len() == 1 {
                            let arg = self.analyze_expr(&args[0])?;
                            let iter_call = IrExpr::MethodCall { target: Box::new(arg), method: "iter".to_string(), args: vec![] };
                            let max_call = IrExpr::MethodCall { target: Box::new(iter_call), method: "max".to_string(), args: vec![] };
                            let copied_call = IrExpr::MethodCall { target: Box::new(max_call), method: "cloned".to_string(), args: vec![] };
                            let unwrap_call = IrExpr::MethodCall { target: Box::new(copied_call), method: "unwrap".to_string(), args: vec![] };
                            return Ok(unwrap_call);
                        }
                        
                        let mut ir_args = Vec::new();
                        for a in args {
                            let ir_arg = self.analyze_expr(a)?;
                            let ty = self.infer_type(a);
                            if !ty.is_copy() {
                                ir_args.push(IrExpr::MethodCall {
                                    target: Box::new(ir_arg),
                                    method: "clone".to_string(),
                                    args: vec![],
                                });
                            } else {
                                ir_args.push(ir_arg);
                            }
                        }
                        
                        let func_name = if name == "main" { "user_main".to_string() } else { name.clone() };
                        Ok(IrExpr::Call {
                            func: func_name, 
                            args: ir_args,
                        })
                    }
                    Expr::Attribute { value, attr } => {
                        let method_name = match attr.as_str() {
                            "append" => "push",
                            _ => attr.as_str(),
                        };
                        let ir_target = self.analyze_expr(value)?;
                        let mut ir_args = Vec::new();
                        for a in args {
                            ir_args.push(self.analyze_expr(a)?);
                        }
                        Ok(IrExpr::MethodCall {
                            target: Box::new(ir_target),
                            method: method_name.to_string(),
                            args: ir_args,
                        })
                    }
                    _ => Err(TsuchinokoError::SemanticError {
                        message: format!("Complex function calls not supported yet: {:?}", func),
                    })
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
            Expr::ListComp { elt, target, iter } => {
                let ir_iter = self.analyze_expr(iter)?;
                self.scope.push();
                self.scope.define(target, Type::Unknown, false);
                let ir_elt = self.analyze_expr(elt)?;
                self.scope.pop();
                Ok(IrExpr::ListComp {
                    elt: Box::new(ir_elt),
                    target: target.clone(),
                    iter: Box::new(ir_iter),
                })
            }
            Expr::Tuple(elements) => {
                let mut ir_elements = Vec::new();
                for e in elements {
                    ir_elements.push(self.analyze_expr(e)?);
                }
                Ok(IrExpr::Tuple(ir_elements))
            }
            Expr::Index { target, index } => {
                let ir_target = self.analyze_expr(target)?;
                let ir_index = self.analyze_expr(index)?;
                Ok(IrExpr::Index {
                    target: Box::new(ir_target),
                    index: Box::new(ir_index),
                })
            }
            Expr::Attribute { value, attr } => {
                // Standalone attribute access (not call)
                // Could be field access.
                let ir_target = self.analyze_expr(value)?;
                Ok(IrExpr::FieldAccess {
                    target: Box::new(ir_target),
                    field: attr.clone(),
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
                    };
                    Ok(IrExpr::UnaryOp {
                        op: ir_op,
                        operand: Box::new(ir_operand),
                    })
                }
            }
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
            Expr::Index { target, index: _ } => {
                let target_ty = self.infer_type(target);
                if let Type::List(inner) = target_ty {
                    *inner
                } else {
                    Type::Unknown
                }
            }
            Expr::Call { func, args: _ } => {
                 // Try to resolve return type
                 if let Expr::Ident(name) = func.as_ref() {
                     if let Some(info) = self.scope.lookup(name) {
                         if let Type::Func { params: _, ret } = &info.ty {
                             return *ret.clone();
                         }
                     }
                 }
                 Type::Unknown
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
            AstBinOp::FloorDiv => IrBinOp::FloorDiv,
            AstBinOp::Mod => IrBinOp::Mod,
            AstBinOp::Pow => IrBinOp::Pow,
            AstBinOp::Eq => IrBinOp::Eq,
            AstBinOp::NotEq => IrBinOp::NotEq,
            AstBinOp::Lt => IrBinOp::Lt,
            AstBinOp::Gt => IrBinOp::Gt,
            AstBinOp::LtEq => IrBinOp::LtEq,
            AstBinOp::GtEq => IrBinOp::GtEq,
            AstBinOp::And => IrBinOp::And,
            AstBinOp::Or => IrBinOp::Or,
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
