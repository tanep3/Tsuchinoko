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
    current_return_type: Option<Type>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            scope: ScopeStack::new(),
            current_return_type: None,
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<Vec<IrNode>, TsuchinokoError> {
        let mut ir_nodes = Vec::new();
        
        // First pass: Find def main() and store its body for potential inlining
        let mut main_func_body: Option<&Vec<Stmt>> = None;
        for stmt in &program.statements {
            if let Stmt::FuncDef { name, params, body, .. } = stmt {
                if name == "main" && params.is_empty() {
                    main_func_body = Some(body);
                    break;
                }
            }
        }
        
        for stmt in &program.statements {
            // Check for if __name__ == "__main__" pattern
            if let Stmt::If { condition, then_body, elif_clauses, else_body } = stmt {
                if elif_clauses.is_empty() && else_body.is_none() {
                    if let Expr::BinOp { left, op, right } = condition {
                        if let (Expr::Ident(l), AstBinOp::Eq, Expr::StringLiteral(r)) = (left.as_ref(), op, right.as_ref()) {
                            if l == "__name__" && r == "__main__" {
                                // Check if body is single main() call and we have a def main()
                                let is_simple_main_call = then_body.len() == 1 && matches!(
                                    &then_body[0],
                                    Stmt::Expr(Expr::Call { func, args }) 
                                    if matches!(func.as_ref(), Expr::Ident(n) if n == "main") && args.is_empty()
                                );
                                
                                if is_simple_main_call {
                                    if let Some(main_body) = main_func_body {
                                        // Inline: emit fn main() with def main()'s body directly
                                        self.scope.push();
                                        let body_nodes = self.analyze_stmts(main_body)?;
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
                                
                                // Fallback: Convert if block body to fn main()
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
            
            // Skip def main() if it was inlined
            if let Stmt::FuncDef { name, params, .. } = stmt {
                if name == "main" && params.is_empty() && main_func_body.is_some() {
                    // Check if we have an if __name__ block that would inline this
                    let has_main_guard = program.statements.iter().any(|s| {
                        if let Stmt::If { condition, then_body, elif_clauses, else_body } = s {
                            if elif_clauses.is_empty() && else_body.is_none() {
                                if let Expr::BinOp { left, op, right } = condition {
                                    if let (Expr::Ident(l), AstBinOp::Eq, Expr::StringLiteral(r)) = (left.as_ref(), op, right.as_ref()) {
                                        if l == "__name__" && r == "__main__" {
                                            return then_body.len() == 1 && matches!(
                                                &then_body[0],
                                                Stmt::Expr(Expr::Call { func, args }) 
                                                if matches!(func.as_ref(), Expr::Ident(n) if n == "main") && args.is_empty()
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        false
                    });
                    if has_main_guard {
                        continue; // Skip emitting def main() as it's inlined
                    }
                }
            }
            
            ir_nodes.push(self.analyze_stmt(stmt)?);
        }
        
        Ok(ir_nodes)
    }

    /// Recursively collect mutations from a statement (including nested blocks)
    fn collect_mutations(
        &self,
        stmt: &Stmt,
        reassigned_vars: &mut std::collections::HashSet<String>,
        mutated_vars: &mut std::collections::HashSet<String>,
        seen_vars: &mut std::collections::HashSet<String>,
    ) {
        fn extract_base_var(expr: &Expr) -> Option<String> {
            match expr {
                Expr::Ident(name) => Some(name.clone()),
                Expr::Index { target, .. } => extract_base_var(target),
                _ => None,
            }
        }

        match stmt {
            // Check for reassignment (x = ... where x already exists)
            Stmt::Assign { target, .. } => {
                let exists_in_scope = self.scope.lookup(target).is_some();
                let seen_in_current_pass = seen_vars.contains(target);
                
                if exists_in_scope || seen_in_current_pass {
                    reassigned_vars.insert(target.clone());
                }
                seen_vars.insert(target.clone());
            }
            // Check for index assignment (x[i] = ...)
            Stmt::IndexAssign { target, .. } => {
                if let Some(name) = extract_base_var(target) {
                    mutated_vars.insert(name);
                }
            }
            // Check for method calls that mutate
            Stmt::Expr(Expr::Call { func, .. }) => {
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Some(name) = extract_base_var(value.as_ref()) {
                        if matches!(attr.as_str(), "append" | "extend" | "push" | "pop" | "insert" | "remove" | "clear") {
                            mutated_vars.insert(name);
                        }
                    }
                }
            }
            // Recurse into for loop body
            Stmt::For { body, .. } => {
                for s in body {
                    self.collect_mutations(s, reassigned_vars, mutated_vars, seen_vars);
                }
            }
            // Recurse into while loop body
            Stmt::While { body, .. } => {
                for s in body {
                    self.collect_mutations(s, reassigned_vars, mutated_vars, seen_vars);
                }
            }
            // Recurse into if/elif/else bodies
            Stmt::If { then_body, elif_clauses, else_body, .. } => {
                for s in then_body {
                    self.collect_mutations(s, reassigned_vars, mutated_vars, seen_vars);
                }
                for (_, elif_body) in elif_clauses {
                    for s in elif_body {
                        self.collect_mutations(s, reassigned_vars, mutated_vars, seen_vars);
                    }
                }
                if let Some(eb) = else_body {
                    for s in eb {
                        self.collect_mutations(s, reassigned_vars, mutated_vars, seen_vars);
                    }
                }
            }
            _ => {}
        }
    }

    /// Analyze a list of statements with lookahead for dead variable elimination
    fn analyze_stmts(&mut self, stmts: &[Stmt]) -> Result<Vec<IrNode>, TsuchinokoError> {
        // First pass: collect variables that are reassigned or mutated
        let mut reassigned_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut mutated_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut seen_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        for stmt in stmts {
            self.collect_mutations(stmt, &mut reassigned_vars, &mut mutated_vars, &mut seen_vars);
        }
        
        let mut ir_nodes = Vec::new();
        
        for (i, stmt) in stmts.iter().enumerate() {
            // Check if this is a variable declaration that will be shadowed by a later for loop
            if let Stmt::Assign { target, value, .. } = stmt {
                // Check if value is a simple literal (0, empty, etc.)
                let is_dead_init = matches!(value, Expr::IntLiteral(0));
                
                if is_dead_init {
                    // Recursively search for ANY for loop with same target
                    fn find_for_loop_with_target(stmts: &[Stmt], target: &str) -> bool {
                        for s in stmts {
                            match s {
                                Stmt::For { target: for_target, body, .. } => {
                                    if for_target == target {
                                        return true;
                                    }
                                    if find_for_loop_with_target(body, target) {
                                        return true;
                                    }
                                }
                                Stmt::While { body, .. } => {
                                    if find_for_loop_with_target(body, target) {
                                        return true;
                                    }
                                }
                                Stmt::If { then_body, elif_clauses, else_body, .. } => {
                                    if find_for_loop_with_target(then_body, target) {
                                        return true;
                                    }
                                    for (_, eb) in elif_clauses {
                                        if find_for_loop_with_target(eb, target) {
                                            return true;
                                        }
                                    }
                                    if let Some(eb) = else_body {
                                        if find_for_loop_with_target(eb, target) {
                                            return true;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        false
                    }
                    
                    if find_for_loop_with_target(&stmts[i+1..], target) {
                        // Skip this dead initialization
                        self.scope.define(target, Type::Int, false);
                        continue;
                    }
                }
            }
            
            // Analyze with mutability info
            let ir_node = self.analyze_stmt_with_mut_info(stmt, &reassigned_vars, &mutated_vars)?;
            ir_nodes.push(ir_node);
        }
        
        Ok(ir_nodes)
    }

    /// Analyze statement with mutability information from lookahead
    fn analyze_stmt_with_mut_info(
        &mut self,
        stmt: &Stmt,
        reassigned_vars: &std::collections::HashSet<String>,
        mutated_vars: &std::collections::HashSet<String>,
    ) -> Result<IrNode, TsuchinokoError> {
        match stmt {
            Stmt::Assign { target, type_hint, value } => {
                let ty = match type_hint {
                    Some(th) => self.type_from_hint(th),
                    None => self.infer_type(value),
                };
                
                // Check if variable is already defined (re-assignment)
                let is_reassign = self.scope.lookup(target).is_some();
                
                // Smart mutability: only mark as mutable if:
                // 1. It's reassigned later, OR
                // 2. It's mutated via method calls (push/append/etc), OR
                // 3. It's mutated via index assignment
                let will_be_reassigned = reassigned_vars.contains(target);
                let will_be_mutated = mutated_vars.contains(target);
                let should_be_mutable = is_reassign || will_be_reassigned || will_be_mutated;
                
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
            // For other statements, delegate to the regular analyze_stmt
            _ => self.analyze_stmt(stmt),
        }
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
                    // Dead code elimination: If initial value is 0 or empty and type is Int/Float,
                    // and the variable will be shadowed by a for loop, emit a no-op.
                    // However, semantic analysis doesn't have lookahead.
                    // We'll mark this variable as "potentially shadowed" and handle in emit phase.
                    // For now, just emit the declaration.
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
                 let ret_type = return_type.as_ref()
                     .map(|th| self.type_from_hint(th))
                     .unwrap_or(Type::Unit);
                 
                 let mut param_types = Vec::new();
                 for p in params {
                     let ty = p.type_hint.as_ref()
                         .map(|th| self.type_from_hint(th))
                         .unwrap_or(Type::Unknown);
                     // Apply Ref transformation for signature
                      let scope_ty = if let Type::List(_) = ty {
                          Type::Ref(Box::new(ty.clone()))
                      } else {
                          ty.clone()
                      };
                     param_types.push(scope_ty);
                 }
                 
                 // Define function in current scope BEFORE analyzing body (for recursion)
                 self.scope.define(name, Type::Func { params: param_types.clone(), ret: Box::new(ret_type.clone()) }, false);

                 self.scope.push();
                 
                 // Add parameters to scope
                 let mut ir_params = Vec::new();
                 for (i, p) in params.iter().enumerate() {
                      let ty = &param_types[i];
                      self.scope.define(&p.name, ty.clone(), false);
                      ir_params.push((p.name.clone(), ty.clone()));
                 }
                 
                 // Store the return type for use in Return statement processing
                 let old_return_type = self.current_return_type.take();
                 self.current_return_type = Some(ret_type.clone());
                 
                 let ir_body = self.analyze_stmts(body)?;
                 
                 // Restore old return type
                 self.current_return_type = old_return_type;
                 
                 self.scope.pop();
                 
                 let ir_name = name.clone();
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
                let mut ir_iter = self.analyze_expr(iter)?;
                let mut iter_type = self.infer_type(iter);

                // If iterating over a Reference to a List, iterate over cloned elements to yield T instead of &T
                if let Type::Ref(inner) = &iter_type {
                    if let Type::List(_) = **inner {
                         ir_iter = IrExpr::MethodCall { target: Box::new(ir_iter), method: "iter".to_string(), args: vec![] };
                         ir_iter = IrExpr::MethodCall { target: Box::new(ir_iter), method: "cloned".to_string(), args: vec![] };
                         iter_type = *inner.clone(); // Now it behaves like the inner list
                    }
                }
                
                let elem_type = if let Type::List(elem) = iter_type {
                    *elem
                } else {
                    Type::Int // Default fallback
                };

                self.scope.push();
                self.scope.define(target, elem_type.clone(), false);
                
                let mut ir_body = Vec::new();
                for s in body {
                    ir_body.push(self.analyze_stmt(s)?);
                }
                self.scope.pop();
                
                Ok(IrNode::For {
                    var: target.clone(),
                    var_type: elem_type,
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
                    Some(e) => {
                        let ir = self.analyze_expr(e)?;
                        let ty = self.infer_type(e);
                        
                        // Check if we're returning from an Optional function
                        let is_optional_return = matches!(
                            &self.current_return_type,
                            Some(Type::Optional(_))
                        );
                        
                        // If returning a Reference to a List (slice), use .to_vec() to return owned
                        let ir = if let Type::Ref(inner) = &ty {
                            if matches!(inner.as_ref(), Type::List(_)) {
                                IrExpr::MethodCall {
                                    target: Box::new(ir),
                                    method: "to_vec".to_string(),
                                    args: vec![],
                                }
                            } else {
                                IrExpr::MethodCall {
                                    target: Box::new(ir),
                                    method: "clone".to_string(),
                                    args: vec![],
                                }
                            }
                        } else {
                            ir
                        };
                        
                        // Wrap in Some() if returning to Optional and value is not None
                        if is_optional_return && !matches!(ir, IrExpr::NoneLit) {
                            Some(Box::new(IrExpr::Call {
                                func: "Some".to_string(),
                                args: vec![ir],
                            }))
                        } else {
                            Some(Box::new(ir))
                        }
                    },
                    None => None,
                };
                Ok(IrNode::Return(ir_expr))
            }
            Stmt::Expr(expr) => {
                let ir_expr = self.analyze_expr(expr)?;
                Ok(IrNode::Expr(ir_expr))
            }
            Stmt::ClassDef { name, fields } => {
                // Convert AST fields to IR fields with types
                let ir_fields: Vec<(String, Type)> = fields
                    .iter()
                    .map(|f| {
                        let ty = self.type_from_hint(&f.type_hint);
                        (f.name.clone(), ty)
                    })
                    .collect();
                
                // Register this struct type in scope (for use in type hints)
                self.scope.define(name, Type::Struct(name.clone()), false);
                
                Ok(IrNode::StructDef {
                    name: name.clone(),
                    fields: ir_fields,
                })
            }
        }
    }

    fn analyze_expr(&mut self, expr: &Expr) -> Result<IrExpr, TsuchinokoError> {
        match expr {
            Expr::IntLiteral(n) => Ok(IrExpr::IntLit(*n)),
            Expr::FloatLiteral(f) => Ok(IrExpr::FloatLit(*f)),
            Expr::StringLiteral(s) => Ok(IrExpr::StringLit(s.clone())),
            Expr::BoolLiteral(b) => Ok(IrExpr::BoolLit(*b)),
            Expr::NoneLiteral => Ok(IrExpr::NoneLit),
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
                            // Use .to_vec() for slice compatibility (&[T] -> Vec<T>)
                            let arg = self.analyze_expr(&args[0])?;
                            return Ok(IrExpr::MethodCall { target: Box::new(arg), method: "to_vec".to_string(), args: vec![] });
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
                        
                        let mut expected_param_types = vec![];
                        if let Some(info) = self.scope.lookup(name) {
                             if let Type::Func { params, .. } = &info.ty {
                                 expected_param_types = params.clone();
                             }
                        }

                        let mut ir_args = Vec::new();
                        for (i, a) in args.iter().enumerate() {
                            let ir_arg = self.analyze_expr(a)?;
                            let actual_ty = self.infer_type(a);
                            let expected_ty = expected_param_types.get(i).cloned().unwrap_or(Type::Unknown);
                            
                            // Check for Auto-Ref: owned -> ref
                            if let Type::Ref(inner) = &expected_ty {
                                if actual_ty == **inner {
                                     ir_args.push(IrExpr::Reference { target: Box::new(ir_arg) });
                                     continue;
                                }
                            }

                            if !actual_ty.is_copy() {
                                let method_name = if let IrExpr::StringLit(_) = ir_arg {
                                    "to_string"
                                } else {
                                    "clone"
                                };
                                ir_args.push(IrExpr::MethodCall {
                                    target: Box::new(ir_arg),
                                    method: method_name.to_string(),
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
            Expr::Dict(entries) => {
                let mut ir_entries = Vec::new();
                for (k, v) in entries {
                    ir_entries.push((self.analyze_expr(k)?, self.analyze_expr(v)?));
                }
                let (key_type, value_type) = if let Some((k, v)) = entries.first() {
                    (self.infer_type(k), self.infer_type(v))
                } else {
                    (Type::Unknown, Type::Unknown)
                };
                Ok(IrExpr::Dict {
                    key_type,
                    value_type,
                    entries: ir_entries,
                })
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
            Expr::BinOp { left, op, right: _ } => {
                match op {
                    AstBinOp::Add | AstBinOp::Sub | AstBinOp::Mul | AstBinOp::Div | 
                    AstBinOp::FloorDiv | AstBinOp::Mod | AstBinOp::Pow => {
                        self.infer_type(left)
                    }
                    AstBinOp::Eq | AstBinOp::NotEq | AstBinOp::Lt | AstBinOp::Gt | 
                    AstBinOp::LtEq | AstBinOp::GtEq | AstBinOp::And | AstBinOp::Or | AstBinOp::In => {
                        Type::Bool
                    }
                }
            }
            Expr::UnaryOp { op, operand } => {
                match op {
                    AstUnaryOp::Neg | AstUnaryOp::Pos => self.infer_type(operand),
                    AstUnaryOp::Not => Type::Bool,
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
            AstBinOp::In => IrBinOp::Contains,
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
