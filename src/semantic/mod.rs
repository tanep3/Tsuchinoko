//! Semantic analysis module

mod scope;
mod types;

pub use scope::*;
pub use types::*;

use crate::error::TsuchinokoError;
use crate::ir::{IrAugAssignOp, IrBinOp, IrExpr, IrNode, IrUnaryOp};
use crate::parser::{
    AugAssignOp, BinOp as AstBinOp, Expr, Program, Stmt, TypeHint, UnaryOp as AstUnaryOp,
};

/// Analyze a program and convert to IR
pub fn analyze(program: &Program) -> Result<Vec<IrNode>, TsuchinokoError> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program)
}

/// Semantic analyzer
pub struct SemanticAnalyzer {
    scope: ScopeStack,
    current_return_type: Option<Type>,
    /// Struct name -> Vec of (field_name, field_type) for constructor type checking
    struct_field_types: std::collections::HashMap<String, Vec<(String, Type)>>,
    /// Variables that need to be mutable (targets of AugAssign or reassignment)
    mutable_vars: std::collections::HashSet<String>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            scope: ScopeStack::new(),
            current_return_type: None,
            struct_field_types: std::collections::HashMap::new(),
            mutable_vars: std::collections::HashSet::new(),
        }
    }

    pub fn define(&mut self, name: &str, ty: Type, mutable: bool) {
        self.scope.define(name, ty, mutable);
    }

    /// Preprocess top-level statements to normalize main function and guard blocks
    fn preprocess_top_level(&self, stmts: &[Stmt]) -> Vec<Stmt> {
        let mut new_stmts = Vec::new();
        let mut main_func_body: Option<Vec<Stmt>> = None;
        let mut main_inlined = false;

        // Pass 1: Find def main()
        for stmt in stmts {
            if let Stmt::FuncDef {
                name, params, body, ..
            } = stmt
            {
                if name == "main" && params.is_empty() {
                    main_func_body = Some(body.clone());
                    break;
                }
            }
        }

        // Pass 2: Flatten structure
        for stmt in stmts {
            // Check for if __name__ == "__main__"
            if let Stmt::If {
                condition,
                then_body,
                elif_clauses,
                else_body,
            } = stmt
            {
                if elif_clauses.is_empty() && else_body.is_none() {
                    if let Expr::BinOp { left, op, right } = condition {
                        if let (Expr::Ident(l), AstBinOp::Eq, Expr::StringLiteral(r)) =
                            (left.as_ref(), op, right.as_ref())
                        {
                            if l == "__name__" && r == "__main__" {
                                // Check if simple main() call
                                let is_simple_main_call = then_body.len() == 1
                                    && matches!(
                                        &then_body[0],
                                        Stmt::Expr(Expr::Call { func, args, .. })
                                        if matches!(func.as_ref(), Expr::Ident(n) if n == "main") && args.is_empty()
                                    );

                                if is_simple_main_call {
                                    if let Some(body) = main_func_body.as_ref() {
                                        // Inline def main()'s body here
                                        new_stmts.extend(body.clone());
                                        main_inlined = true;
                                    } else {
                                        // Inline the if block's body here
                                        new_stmts.extend(then_body.clone());
                                    }
                                } else {
                                    // Inline the if block's body here
                                    new_stmts.extend(then_body.clone());
                                }
                                continue;
                            }
                        }
                    }
                }
            }
            new_stmts.push(stmt.clone());
        }

        // Remove def main() if it was inlined to avoid duplication
        if main_inlined {
            new_stmts.retain(|s| {
                !matches!(s, Stmt::FuncDef { name, params, .. } if name == "main" && params.is_empty())
            });
        }

        new_stmts
    }

    pub fn analyze(&mut self, program: &Program) -> Result<Vec<IrNode>, TsuchinokoError> {
        // Step 1: Pre-processing (Declarative AST transformation)
        let stmts = self.preprocess_top_level(&program.statements);

        // Step 1.5: Collect mutable variables (targets of AugAssign or reassignment)
        self.collect_mutable_vars(&stmts);

        // Step 2: Unified Analysis (Pass 0 -> Pass 1)
        // Now top-level statements are treated exactly like block statements
        let ir_nodes = self.analyze_stmts(&stmts)?;

        // Step 3: Wrap top-level statements in a main function if they are loose statements
        // (The Emitter expects entry point. We should check if we need to wrap them or if Emitter handles it)
        // Historically, Tsuchinoko's `analyze` returned FuncDecl for main.
        // But `analyze_stmts` returns list of nodes.
        // If we just return nodes, the emitter needs to know these are top level.
        // Let's wrap loose statements (Vars, Exprs, Assigns) into a FuncDecl "main" if they exist?
        // Actually, the previous `analyze` implementation explicitly created `IrNode::FuncDecl { name: "main" }`.

        // Let's group loose statements into a main function to preserve behavior.
        let mut main_body = Vec::new();
        let mut other_decls = Vec::new();

        for node in ir_nodes {
            match node {
                IrNode::FuncDecl { .. }
                | IrNode::StructDef { .. }
                | IrNode::TypeAlias { .. }
                | IrNode::ImplBlock { .. }
                | IrNode::Sequence(_) => {
                    other_decls.push(node);
                }
                _ => {
                    main_body.push(node);
                }
            }
        }

        if !main_body.is_empty() {
            other_decls.push(IrNode::FuncDecl {
                name: "main".to_string(),
                params: vec![],
                ret: Type::Unit,
                body: main_body,
            });
        }

        Ok(other_decls)
    }

    /// Collect all variables that need to be mutable (targets of AugAssign or reassignment)
    fn collect_mutable_vars(&mut self, stmts: &[Stmt]) {
        let mut reassigned_vars = std::collections::HashSet::new();
        let mut mutated_vars = std::collections::HashSet::new();
        let mut seen_vars = std::collections::HashSet::new();

        for stmt in stmts {
            self.collect_mutations(
                stmt,
                &mut reassigned_vars,
                &mut mutated_vars,
                &mut seen_vars,
            );
        }

        // Store in mutable_vars field for later use
        self.mutable_vars = reassigned_vars.union(&mutated_vars).cloned().collect();
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
                        if matches!(
                            attr.as_str(),
                            "append" | "extend" | "push" | "pop" | "insert" | "remove" | "clear"
                        ) {
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
            Stmt::If {
                then_body,
                elif_clauses,
                else_body,
                ..
            } => {
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
            // Check for augmented assignment (x += ..., x -= ..., etc.)
            Stmt::AugAssign { target, .. } => {
                // AugAssign target needs to be mutable
                reassigned_vars.insert(target.clone());
            }
            // Check for IndexSwap (a[i], a[j] = a[j], a[i])
            Stmt::IndexSwap { left_targets, .. } => {
                for target in left_targets {
                    if let Some(name) = extract_base_var(target) {
                        mutated_vars.insert(name);
                    }
                }
            }
            _ => {}
        }
    }

    /// Convert an expression to a type (for type aliases like ConditionFunction = Callable[...])
    fn expr_to_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Ident(name) => Some(self.type_from_hint(&TypeHint {
                name: name.clone(),
                params: vec![],
            })),
            Expr::Index { target, index } => {
                if let Expr::Ident(name) = target.as_ref() {
                    // Handle Callable[[params], ret]
                    if name == "Callable" {
                        // Parse index as tuple/list of types
                        let params_expr = &index; // Usually Tuple(List([p1, p2]), ret)

                        if let Expr::Tuple(elements) = params_expr.as_ref() {
                            if elements.len() >= 2 {
                                // Param types (List)
                                let param_list_expr = &elements[0];
                                let ret_expr = &elements[1];

                                let mut param_types = Vec::new();
                                if let Expr::List(p_elems) = param_list_expr {
                                    for p in p_elems {
                                        if let Some(t) = self.expr_to_type(p) {
                                            param_types.push(t);
                                        } else {
                                            return Some(Type::Unknown);
                                        }
                                    }
                                } else {
                                    // Fallback: if it's not a list, maybe it's a single type or ellipsis?
                                    if let Some(t) = self.expr_to_type(param_list_expr) {
                                        param_types.push(t);
                                    }
                                }

                                let ret_type = self.expr_to_type(ret_expr).unwrap_or(Type::Unknown);

                                return Some(Type::Func {
                                    params: param_types,
                                    ret: Box::new(ret_type),
                                    is_boxed: true,
                                });
                            }
                        }
                    }

                    // Handle Dict[K, V]
                    if name == "Dict" || name == "dict" {
                        if let Expr::Tuple(elements) = index.as_ref() {
                            if elements.len() >= 2 {
                                let key_ty =
                                    self.expr_to_type(&elements[0]).unwrap_or(Type::Unknown);
                                let val_ty =
                                    self.expr_to_type(&elements[1]).unwrap_or(Type::Unknown);
                                return Some(Type::Dict(Box::new(key_ty), Box::new(val_ty)));
                            }
                        }
                    }

                    if name == "List" || name == "list" {
                        let inner = self.expr_to_type(index).unwrap_or(Type::Unknown);
                        return Some(Type::List(Box::new(inner)));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Analyze a list of statements with lookahead for dead variable elimination
    fn analyze_stmts(&mut self, stmts: &[Stmt]) -> Result<Vec<IrNode>, TsuchinokoError> {
        // Collect mutations first (pass 0)
        let mut reassigned_vars: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut mutated_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut seen_vars: std::collections::HashSet<String> = std::collections::HashSet::new();

        for stmt in stmts {
            self.collect_mutations(
                stmt,
                &mut reassigned_vars,
                &mut mutated_vars,
                &mut seen_vars,
            );
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
                                Stmt::For {
                                    target: for_target,
                                    body,
                                    ..
                                } => {
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
                                Stmt::If {
                                    then_body,
                                    elif_clauses,
                                    else_body,
                                    ..
                                } => {
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

                    if find_for_loop_with_target(&stmts[i + 1..], target) {
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
            Stmt::Assign {
                target,
                type_hint,
                value,
            } => {
                // Check if this looks like a Type Alias (Capitalized target = TypeExpr)
                if type_hint.is_none()
                    && target
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false)
                {
                    if let Some(ty) = self.expr_to_type(value) {
                        self.scope.define(target, ty.clone(), false);
                        return Ok(IrNode::TypeAlias {
                            name: target.clone(),
                            ty,
                        });
                    }
                }

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
                // 4. It's a Struct or List (which often need mutation)
                let will_be_reassigned = reassigned_vars.contains(target);
                let will_be_mutated = mutated_vars.contains(target);
                let is_mutable_type = matches!(ty, Type::List(_) | Type::Struct(_));
                let should_be_mutable =
                    is_reassign || will_be_reassigned || will_be_mutated || is_mutable_type;

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
            Stmt::Assign {
                target,
                type_hint,
                value,
            } => {
                // Handle self.field = value pattern
                if target.starts_with("self.") {
                    let field_name = target.trim_start_matches("self.");
                    // Strip dunder prefix for Rust struct field
                    let rust_field_name = field_name.trim_start_matches("__").to_string();
                    let ir_value = self.analyze_expr(value)?;
                    return Ok(IrNode::FieldAssign {
                        target: Box::new(IrExpr::Var("self".to_string())),
                        field: rust_field_name,
                        value: Box::new(ir_value),
                    });
                }

                // Check for TypeAlias even in top-level analyze_stmt
                if type_hint.is_none()
                    && target
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false)
                {
                    if let Some(ty) = self.expr_to_type(value) {
                        self.scope.define(target, ty.clone(), false);
                        return Ok(IrNode::TypeAlias {
                            name: target.clone(),
                            ty,
                        });
                    }
                }
                let ty = match type_hint {
                    Some(th) => self.type_from_hint(th),
                    None => self.infer_type(value),
                };

                // Check if variable is already defined (re-assignment)
                let is_reassign = self.scope.lookup(target).is_some();

                // In Python, lists are always mutable. In Rust, we should make them mutable by default
                // to allow modification (like push, index assign).
                // Structs should also be mutable to allow &mut self method calls.
                // Dicts need to be mutable for insert().
                // Also respect re-assignment.
                let should_be_mutable =
                    is_reassign || matches!(ty, Type::List(_) | Type::Struct(_) | Type::Dict(_, _));

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
            Stmt::IndexAssign {
                target,
                index,
                value,
            } => {
                let target_ty = self.infer_type(target);
                let ir_target = self.analyze_expr(target)?;
                let ir_index = self.analyze_expr(index)?;
                let ir_value = self.analyze_expr(value)?;

                // For Dict types, use insert() method instead of index assignment
                if matches!(target_ty, Type::Dict(_, _)) {
                    Ok(IrNode::Expr(IrExpr::MethodCall {
                        target: Box::new(ir_target),
                        method: "insert".to_string(),
                        args: vec![ir_index, ir_value],
                    }))
                } else {
                    Ok(IrNode::IndexAssign {
                        target: Box::new(ir_target),
                        index: Box::new(ir_index),
                        value: Box::new(ir_value),
                    })
                }
            }
            Stmt::AugAssign { target, op, value } => {
                // Convert augmented assignment (x += 1) to IR
                let ir_value = self.analyze_expr(value)?;
                let ir_op = match op {
                    AugAssignOp::Add => IrAugAssignOp::Add,
                    AugAssignOp::Sub => IrAugAssignOp::Sub,
                    AugAssignOp::Mul => IrAugAssignOp::Mul,
                    AugAssignOp::Div => IrAugAssignOp::Div,
                    AugAssignOp::FloorDiv => IrAugAssignOp::FloorDiv,
                    AugAssignOp::Mod => IrAugAssignOp::Mod,
                };
                Ok(IrNode::AugAssign {
                    target: target.clone(),
                    op: ir_op,
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
            Stmt::IndexSwap {
                left_targets,
                right_values,
            } => {
                // Handle swap pattern: a[i], a[j] = a[j], a[i]
                // Convert to Rust: a.swap(i as usize, j as usize)
                // This only works for simple 2-element swaps on the same array

                if left_targets.len() == 2 && right_values.len() == 2 {
                    // Check if this is a simple swap pattern:
                    // left_targets[0] matches right_values[1] and left_targets[1] matches right_values[0]
                    if let (
                        Expr::Index {
                            target: t1,
                            index: i1,
                        },
                        Expr::Index {
                            target: t2,
                            index: i2,
                        },
                    ) = (&left_targets[0], &left_targets[1])
                    {
                        // Check if targets are the same array
                        if format!("{t1:?}") == format!("{t2:?}") {
                            // Generate: target.swap(i1 as usize, i2 as usize)
                            let ir_target = self.analyze_expr(t1)?;
                            let ir_i1 = self.analyze_expr(i1)?;
                            let ir_i2 = self.analyze_expr(i2)?;

                            // Cast indices to usize
                            let i1_cast = IrExpr::Cast {
                                target: Box::new(ir_i1),
                                ty: "usize".to_string(),
                            };
                            let i2_cast = IrExpr::Cast {
                                target: Box::new(ir_i2),
                                ty: "usize".to_string(),
                            };

                            return Ok(IrNode::Expr(IrExpr::MethodCall {
                                target: Box::new(ir_target),
                                method: "swap".to_string(),
                                args: vec![i1_cast, i2_cast],
                            }));
                        }
                    }
                }

                // Fallback: not a simple swap, generate temp variable approach
                // For now, return an error for unsupported patterns
                Err(TsuchinokoError::TypeError {
                    line: 0,
                    message: "Unsupported swap pattern - only a[i], a[j] = a[j], a[i] is supported"
                        .to_string(),
                })
            }
            Stmt::FuncDef {
                name,
                params,
                return_type,
                body,
            } => {
                let ret_type = return_type
                    .as_ref()
                    .map(|th| self.type_from_hint(th))
                    .unwrap_or(Type::Unit);

                // Collect mutations in the function body to detect which params need &mut
                let mut reassigned_vars = std::collections::HashSet::new();
                let mut mutated_vars = std::collections::HashSet::new();
                let mut seen_vars = std::collections::HashSet::new();
                for stmt in body {
                    self.collect_mutations(
                        stmt,
                        &mut reassigned_vars,
                        &mut mutated_vars,
                        &mut seen_vars,
                    );
                }

                let mut param_types = Vec::new();
                for p in params {
                    let ty = p
                        .type_hint
                        .as_ref()
                        .map(|th| self.type_from_hint(th))
                        .unwrap_or(Type::Unknown);
                    // In Rust, we pass objects by reference.
                    // So if ty is List/Dict/Struct/String/Tuple, the function signature should reflect Ref(ty).
                    // If the parameter is mutated in the function body, use MutRef instead.
                    let is_mutated = mutated_vars.contains(&p.name);
                    let mut signature_ty = match &ty {
                        Type::List(_)
                        | Type::Dict(_, _)
                        | Type::Struct(_)
                        | Type::String
                        | Type::Tuple(_) => {
                            if is_mutated {
                                Type::MutRef(Box::new(ty.clone()))
                            } else {
                                Type::Ref(Box::new(ty.clone()))
                            }
                        }
                        _ => ty.clone(),
                    };

                    // Critical fix for closures: If the function returns a boxed closure,
                    // we need to pass parameters by value (owned) to avoid lifetime issues ('static)
                    if matches!(ret_type, Type::Func { is_boxed: true, .. }) {
                        signature_ty = ty.clone();
                    }

                    param_types.push(signature_ty);
                }

                // If the return type is a Type::Struct (Alias), check if it resolves to a Func.
                // If so, use the Func's return type as the function return type.
                let mut resolved_ret_type = ret_type.clone();
                if let Type::Struct(alias_name) = &ret_type {
                    if let Some(info) = self.scope.lookup(alias_name) {
                        if let Type::Func { ret, .. } = &info.ty {
                            resolved_ret_type = *ret.clone();
                        }
                    }
                }

                // Define function in current scope BEFORE analyzing body (for recursion)
                self.scope.define(
                    name,
                    Type::Func {
                        params: param_types.clone(),
                        ret: Box::new(resolved_ret_type.clone()),
                        is_boxed: false,
                    },
                    false,
                );

                // Check if nested function (Closure conversion)
                if self.scope.depth() > 0 {
                    self.scope.push();

                    // Add parameters to scope
                    let mut param_names = Vec::new();

                    for (i, p) in params.iter().enumerate() {
                        let ty = &param_types[i];
                        self.scope.define(&p.name, ty.clone(), false);
                        param_names.push(p.name.clone());
                    }

                    let ir_body = self.analyze_stmts(body)?;
                    self.scope.pop();

                    // Warn about closures if capturing variables?
                    // Currently implicit capture via 'move' in Rust.

                    let closure = IrExpr::Closure {
                        params: param_names,
                        body: ir_body,
                        ret_type: ret_type.clone(),
                    };

                    // Wrap closure in Box::new(...) to match Type::Func (Box<dyn Fn...>)
                    let boxed_closure = IrExpr::BoxNew(Box::new(closure));

                    return Ok(IrNode::VarDecl {
                        name: name.clone(),
                        ty: Type::Func {
                            params: param_types,
                            ret: Box::new(resolved_ret_type),
                            is_boxed: true,
                        }, // Variable holding closure is Boxed
                        mutable: false,
                        init: Some(Box::new(boxed_closure)),
                    });
                }

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

            Stmt::If {
                condition,
                then_body,
                elif_clauses,
                else_body,
            } => {
                // Check for main block
                if let Expr::BinOp { left, op, right } = condition {
                    if let (Expr::Ident(l), AstBinOp::Eq, Expr::StringLiteral(r)) =
                        (left.as_ref(), op, right.as_ref())
                    {
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
                        ir_iter = IrExpr::MethodCall {
                            target: Box::new(ir_iter),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        ir_iter = IrExpr::MethodCall {
                            target: Box::new(ir_iter),
                            method: "cloned".to_string(),
                            args: vec![],
                        };
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

                // Use analyze_stmts to properly detect mutable variables in loop body
                let ir_body = self.analyze_stmts(body)?;
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
                // Use analyze_stmts to properly detect mutable variables in loop body
                let ir_body = self.analyze_stmts(body)?;
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
                        let is_optional_return =
                            matches!(&self.current_return_type, Some(Type::Optional(_)));

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
                                func: Box::new(IrExpr::Var("Some".to_string())),
                                args: vec![ir],
                            }))
                        } else {
                            Some(Box::new(ir))
                        }
                    }
                    None => None,
                };
                Ok(IrNode::Return(ir_expr))
            }
            Stmt::Expr(expr) => {
                let ir_expr = self.analyze_expr(expr)?;
                Ok(IrNode::Expr(ir_expr))
            }
            Stmt::ClassDef {
                name,
                fields,
                methods,
            } => {
                // Convert AST fields to IR fields with types
                let ir_fields: Vec<(String, Type)> = fields
                    .iter()
                    .map(|f| {
                        let ty = self.type_from_hint(&f.type_hint);
                        (f.name.clone(), ty)
                    })
                    .collect();

                // Save field types for constructor type checking
                self.struct_field_types
                    .insert(name.clone(), ir_fields.clone());

                // Register this struct type in scope (for use in type hints)
                self.scope.define(name, Type::Struct(name.clone()), false);

                // If there are methods, create an impl block
                let mut result_nodes = vec![IrNode::StructDef {
                    name: name.clone(),
                    fields: ir_fields.clone(),
                }];

                if !methods.is_empty() {
                    let mut ir_methods = Vec::new();

                    for method in methods {
                        // Skip __init__ - it's handled via fields
                        if method.name == "__init__" {
                            continue;
                        }

                        // Parse method parameters
                        let ir_params: Vec<(String, Type)> = method
                            .params
                            .iter()
                            .map(|p| {
                                let ty = p
                                    .type_hint
                                    .as_ref()
                                    .map(|h| self.type_from_hint(h))
                                    .unwrap_or(Type::Unknown);
                                (p.name.clone(), ty)
                            })
                            .collect();

                        let ret_ty = method
                            .return_type
                            .as_ref()
                            .map(|h| self.type_from_hint(h))
                            .unwrap_or(Type::Unit);

                        // Analyze method body with self in scope
                        self.scope.push();
                        // Define self as this struct type
                        self.scope.define("self", Type::Struct(name.clone()), false);
                        // Define struct fields as self.field_name for type inference
                        for (field_name, field_ty) in &ir_fields {
                            // Strip dunder prefix for consistency
                            let rust_field_name =
                                if field_name.starts_with("__") && !field_name.ends_with("__") {
                                    field_name.trim_start_matches("__")
                                } else {
                                    field_name.as_str()
                                };
                            self.scope.define(
                                &format!("self.{rust_field_name}"),
                                field_ty.clone(),
                                false,
                            );
                        }
                        // Define method params
                        for (p_name, p_ty) in &ir_params {
                            self.scope.define(p_name, p_ty.clone(), false);
                        }

                        let ir_body: Vec<IrNode> = method
                            .body
                            .iter()
                            .map(|s| self.analyze_stmt(s))
                            .collect::<Result<Vec<_>, _>>()?;
                        self.scope.pop();

                        // Check if method modifies self (contains FieldAssign)
                        let takes_mut_self = ir_body
                            .iter()
                            .any(|node| matches!(node, IrNode::FieldAssign { .. }));

                        ir_methods.push(IrNode::MethodDecl {
                            name: method.name.clone(),
                            params: ir_params,
                            ret: ret_ty,
                            body: ir_body,
                            takes_self: !method.is_static,
                            takes_mut_self,
                        });
                    }

                    result_nodes.push(IrNode::ImplBlock {
                        struct_name: name.clone(),
                        methods: ir_methods,
                    });
                }

                // Return node(s) - use Sequence if multiple
                if result_nodes.len() == 1 {
                    Ok(result_nodes.remove(0))
                } else {
                    // Return Sequence containing StructDef + ImplBlock
                    Ok(IrNode::Sequence(result_nodes))
                }
            }
            Stmt::Raise {
                exception_type: _,
                message,
            } => {
                let msg_ir = self.analyze_expr(message)?;
                // Extract string from message
                let msg = if let IrExpr::StringLit(s) = msg_ir {
                    s
                } else {
                    "Error".to_string()
                };
                Ok(IrNode::Panic(msg))
            }
            Stmt::TryExcept {
                try_body,
                except_type: _,
                except_body,
            } => {
                // Analyze try body
                let ir_try_body: Vec<IrNode> = try_body
                    .iter()
                    .map(|s| self.analyze_stmt(s))
                    .collect::<Result<Vec<_>, _>>()?;

                // Analyze except body
                let ir_except_body: Vec<IrNode> = except_body
                    .iter()
                    .map(|s| self.analyze_stmt(s))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(IrNode::TryBlock {
                    try_body: ir_try_body,
                    except_body: ir_except_body,
                })
            }
            Stmt::Break => Ok(IrNode::Break),
            Stmt::Continue => Ok(IrNode::Continue),
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
                // Handle 'in' operator: x in y -> y.contains(&x) or y.contains_key(&x)
                if let AstBinOp::In = op {
                    let right_ty = self.infer_type(right);
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
                    // Assuming for now generic MethodCall.

                    return Ok(IrExpr::MethodCall {
                        target: Box::new(ir_right),
                        method: method.to_string(),
                        args: vec![IrExpr::Reference {
                            target: Box::new(ir_left),
                        }],
                    });
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
                
                // Combine positional args with kwargs values
                // For now, we append kwargs values to the end of args in order
                // A more sophisticated implementation would reorder based on function signature
                let mut all_args: Vec<&Expr> = args.iter().collect();
                for (_, value) in kwargs {
                    all_args.push(value);
                }
                
                match func.as_ref() {
                    Expr::Ident(name) => {
                        // Try built-in function handler first
                        if let Some(ir_expr) = self.try_handle_builtin_call(name, &all_args.iter().cloned().cloned().collect::<Vec<_>>())? {
                            return Ok(ir_expr);
                        }

                        // Check if this is a struct constructor call
                        if let Some(field_types) = self.struct_field_types.get(name).cloned() {
                            // Struct constructor - use field types for Auto-Box
                            let expected_types: Vec<Type> =
                                field_types.iter().map(|(_, ty)| ty.clone()).collect();
                            let args_cloned: Vec<Expr> = all_args.iter().cloned().cloned().collect();
                            let ir_args = self.analyze_call_args(&args_cloned, &expected_types, name)?;
                            return Ok(IrExpr::Call {
                                func: Box::new(IrExpr::Var(name.clone())),
                                args: ir_args,
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
                        let args_cloned: Vec<Expr> = all_args.iter().cloned().cloned().collect();
                        let ir_args = self.analyze_call_args(
                            &args_cloned,
                            &expected_param_types,
                            &self.get_func_name_for_debug(func.as_ref()),
                        )?;

                        let final_func = if name == "main" {
                            Box::new(IrExpr::Var("user_main".to_string()))
                        } else {
                            Box::new(IrExpr::Var(name.clone()))
                        };

                        Ok(IrExpr::Call {
                            func: final_func,
                            args: ir_args,
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
                        let ir_func = self.analyze_expr(func)?;
                        Ok(IrExpr::Call {
                            func: Box::new(ir_func),
                            args: ir_args,
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
            Expr::IfExp { test, body, orelse } => {
                let ir_test = self.analyze_expr(test)?;
                let ir_body = self.analyze_expr(body)?;
                let ir_orelse = self.analyze_expr(orelse)?;

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
                let (final_key_type, final_value_type) = if let Some((k, v)) = entries.first() {
                    (self.infer_type(k), self.infer_type(v))
                } else {
                    (Type::Unknown, Type::Unknown)
                };

                Ok(IrExpr::Dict {
                    key_type: final_key_type,
                    value_type: final_value_type,
                    entries: ir_entries,
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
            Expr::Index { target, index } => {
                let ir_target = self.analyze_expr(target)?;
                let ir_index = self.analyze_expr(index)?;
                Ok(IrExpr::Index {
                    target: Box::new(ir_target),
                    index: Box::new(ir_index),
                })
            }
            Expr::Slice { target, start, end } => {
                // Python slices: nums[:3], nums[-3:], nums[1:len(nums)-1]
                // Rust equivalents depend on the slice type
                let ir_target = self.analyze_expr(target)?;

                let ir_start = match start {
                    Some(s) => Some(Box::new(self.analyze_expr(s)?)),
                    None => None,
                };

                let ir_end = match end {
                    Some(e) => Some(Box::new(self.analyze_expr(e)?)),
                    None => None,
                };

                Ok(IrExpr::Slice {
                    target: Box::new(ir_target),
                    start: ir_start,
                    end: ir_end,
                })
            }
            Expr::Attribute { value, attr } => {
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
                    };
                    Ok(IrExpr::UnaryOp {
                        op: ir_op,
                        operand: Box::new(ir_operand),
                    })
                }
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
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
            Expr::ListComp { elt, .. } | Expr::GenExpr { elt, .. } => {
                Type::List(Box::new(self.infer_type(elt)))
            }
            Expr::Ident(name) => {
                let ty = if let Some(info) = self.scope.lookup(name) {
                    info.ty.clone()
                } else {
                    Type::Unknown
                };
                ty
            }
            Expr::Index { target, index: _ } => {
                let target_ty = self.infer_type(target);
                if let Type::List(inner) = target_ty {
                    *inner
                } else if let Type::Ref(inner) = target_ty {
                    if let Type::List(elem) = *inner {
                        *elem
                    } else {
                        Type::Unknown
                    }
                } else {
                    Type::Unknown
                }
            }
            Expr::Call { func, args, .. } => {
                // Try to resolve return type
                if let Expr::Ident(name) = func.as_ref() {
                    if name == "tuple" || name == "list" {
                        return Type::List(Box::new(Type::Unknown));
                    }
                    // dict(x) returns the same Dict type as x
                    if name == "dict" && !args.is_empty() {
                        let arg_ty = self.infer_type(&args[0]);
                        if let Type::Dict(k, v) = arg_ty {
                            return Type::Dict(k, v);
                        }
                        // If arg is unknown, still return Dict with unknown types
                        return Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown));
                    }
                    if let Some(info) = self.scope.lookup(name) {
                        if let Type::Func { params: _, ret, .. } = &info.ty {
                            return *ret.clone();
                        }
                    }
                } else if let Expr::Attribute { value, attr } = func.as_ref() {
                    let mut target_ty = self.infer_type(value);
                    while let Type::Ref(inner) = target_ty {
                        target_ty = *inner;
                    }
                    match (target_ty, attr.as_str()) {
                        (Type::Dict(k, v), "items") => {
                            return Type::List(Box::new(Type::Tuple(vec![*k.clone(), *v.clone()])))
                        }
                        (Type::Ref(inner), "items")
                            if matches!(inner.as_ref(), Type::Dict(_, _)) =>
                        {
                            if let Type::Dict(k, v) = inner.as_ref() {
                                return Type::List(Box::new(Type::Tuple(vec![
                                    *k.clone(),
                                    *v.clone(),
                                ])));
                            }
                        }
                        (Type::Dict(k, _), "keys") => return Type::List(k.clone()),
                        (Type::Ref(inner), "keys")
                            if matches!(inner.as_ref(), Type::Dict(_, _)) =>
                        {
                            if let Type::Dict(k, _) = inner.as_ref() {
                                return Type::List(k.clone());
                            }
                        }
                        (Type::Dict(_, v), "values") => return Type::List(v.clone()),
                        (Type::Ref(inner), "values")
                            if matches!(inner.as_ref(), Type::Dict(_, _)) =>
                        {
                            if let Type::Dict(_, v) = inner.as_ref() {
                                return Type::List(v.clone());
                            }
                        }
                        (Type::List(inner), "iter") => {
                            return Type::List(Box::new(Type::Ref(inner.clone())))
                        }
                        (Type::Ref(inner), "iter") => {
                            if let Type::List(elem) = inner.as_ref() {
                                return Type::List(Box::new(Type::Ref(elem.clone())));
                            } else if let Type::Dict(k, v) = inner.as_ref() {
                                return Type::List(Box::new(Type::Tuple(vec![
                                    Type::Ref(k.clone()),
                                    Type::Ref(v.clone()),
                                ])));
                            }
                        }
                        (Type::Dict(k, v), "iter") => {
                            return Type::List(Box::new(Type::Tuple(vec![
                                Type::Ref(k.clone()),
                                Type::Ref(v.clone()),
                            ])))
                        }
                        (Type::String, "join") => return Type::String,
                        _ => {}
                    }
                }
                Type::Unknown
            }
            Expr::BinOp { left, op, right: _ } => match op {
                AstBinOp::Add
                | AstBinOp::Sub
                | AstBinOp::Mul
                | AstBinOp::Div
                | AstBinOp::FloorDiv
                | AstBinOp::Mod
                | AstBinOp::Pow => self.infer_type(left),
                AstBinOp::Eq
                | AstBinOp::NotEq
                | AstBinOp::Lt
                | AstBinOp::Gt
                | AstBinOp::LtEq
                | AstBinOp::GtEq
                | AstBinOp::And
                | AstBinOp::Or
                | AstBinOp::In => Type::Bool,
            },
            Expr::UnaryOp { op, operand } => match op {
                AstUnaryOp::Neg | AstUnaryOp::Pos => self.infer_type(operand),
                AstUnaryOp::Not => Type::Bool,
            },
            Expr::IfExp { body, orelse, .. } => {
                let t_body = self.infer_type(body);
                let t_orelse = self.infer_type(orelse);
                if t_body == t_orelse {
                    t_body
                } else if t_body == Type::Unknown {
                    t_orelse
                } else if t_orelse == Type::Unknown {
                    t_body
                } else {
                    Type::Unknown
                }
            }
            Expr::Attribute { value, attr } => {
                // Handle self.field type inference
                if let Expr::Ident(target_name) = value.as_ref() {
                    if target_name == "self" {
                        // Strip dunder prefix for lookup consistency
                        let rust_field = if attr.starts_with("__") && !attr.ends_with("__") {
                            attr.trim_start_matches("__")
                        } else {
                            attr.as_str()
                        };
                        // Look up self.field_name in scope
                        if let Some(info) = self.scope.lookup(&format!("self.{rust_field}")) {
                            return info.ty.clone();
                        }
                    }
                }
                Type::Unknown
            }
            _ => Type::Unknown,
        }
    }

    fn analyze_call_args(
        &mut self,
        args: &[Expr],
        expected_param_types: &[Type],
        _func_name: &str,
    ) -> Result<Vec<IrExpr>, TsuchinokoError> {
        let mut ir_args = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let ir_arg = self.analyze_expr(a)?;
            let actual_ty = self.infer_type(a);
            let expected_ty = expected_param_types
                .get(i)
                .cloned()
                .unwrap_or(Type::Unknown);

            let coerced = self.coerce_arg(ir_arg, &actual_ty, &expected_ty, a);
            ir_args.push(coerced);
        }
        Ok(ir_args)
    }

    /// Coerce a single argument to match the expected type
    /// Handles Auto-Box, Auto-Ref, Auto-Deref, and Fallback Clone
    fn coerce_arg(
        &self,
        mut ir_arg: IrExpr,
        actual_ty: &Type,
        expected_ty: &Type,
        expr: &Expr,
    ) -> IrExpr {
        // 1. Unpack expectation (check if expected type is a reference or mutable reference)
        let (target_ty, needs_ref, needs_mut_ref) = match expected_ty {
            Type::MutRef(inner) => (inner.as_ref().clone(), false, true),
            Type::Ref(inner) => (inner.as_ref().clone(), true, false),
            _ => (expected_ty.clone(), false, false),
        };

        let resolved_target = self.resolve_type(&target_ty);
        let mut resolved_actual = self.resolve_type(actual_ty);

        // Strip all references from actual for comparison
        while let Type::Ref(inner) = resolved_actual {
            resolved_actual = *inner;
        }

        // 2. Auto-Box: Fn -> Box<dyn Fn>
        if let Type::Func { is_boxed: true, .. } = &resolved_target {
            if let Type::Func {
                is_boxed: false, ..
            } = &resolved_actual
            {
                ir_arg = IrExpr::BoxNew(Box::new(ir_arg));

                // If target was a named alias, add explicit cast
                if let Type::Struct(alias_name) = &target_ty {
                    ir_arg = IrExpr::Cast {
                        target: Box::new(ir_arg),
                        ty: alias_name.clone(),
                    };
                }
                return ir_arg;
            }
        }

        // 3. Auto-Ref for Index expressions
        if needs_ref && matches!(expr, Expr::Index { .. }) {
            return IrExpr::Reference {
                target: Box::new(ir_arg),
            };
        }

        // 3.5 Auto-MutRef for mutable reference parameters
        if needs_mut_ref {
            // Need a mutable reference
            return IrExpr::MutReference {
                target: Box::new(ir_arg),
            };
        }

        // 4. Auto-Ref/Deref logic
        if needs_ref {
            // Need a reference
            if let Type::Ref(_) = actual_ty {
                // Already a reference, use as-is
                ir_arg
            } else {
                // Not a reference, add one
                IrExpr::Reference {
                    target: Box::new(ir_arg),
                }
            }
        } else {
            // Need an owned value - apply Auto-Deref for Copy types
            let mut current_ty = actual_ty.clone();
            while let Type::Ref(inner) = &current_ty {
                let inner_ty = inner.as_ref();
                if inner_ty.is_copy() {
                    ir_arg = IrExpr::UnaryOp {
                        op: IrUnaryOp::Deref,
                        operand: Box::new(ir_arg),
                    };
                    current_ty = inner_ty.clone();
                    // If expected type is Unknown or compatible, we're done
                    if resolved_target == Type::Unknown
                        || current_ty.is_compatible_with(&resolved_target)
                    {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Fallback Clone for non-Copy types
            // Skip clone for method calls that return Copy types (like len())
            let is_copy_method =
                matches!(&ir_arg, IrExpr::MethodCall { method, .. } if method == "len");
            if !resolved_actual.is_copy()
                && !matches!(actual_ty, Type::Ref(_))
                && !matches!(resolved_actual, Type::Func { .. })
                && !is_copy_method
            {
                let method = if matches!(ir_arg, IrExpr::StringLit(_) | IrExpr::FString { .. }) {
                    "to_string"
                } else {
                    "clone"
                };
                ir_arg = IrExpr::MethodCall {
                    target: Box::new(ir_arg),
                    method: method.to_string(),
                    args: vec![],
                };
            }

            // Special case: &String -> String
            if let Type::Ref(inner) = actual_ty {
                if **inner == Type::String {
                    ir_arg = IrExpr::MethodCall {
                        target: Box::new(ir_arg),
                        method: "to_string".to_string(),
                        args: vec![],
                    };
                }
            }

            ir_arg
        }
    }

    /// Define loop variables in scope based on iterator type
    /// Used by ListComp, GenExpr, and For loops
    fn define_loop_variables(&mut self, target: &str, iter_ty: &Type, wrap_in_ref: bool) {
        match iter_ty {
            Type::List(inner) => {
                let elem_ty = inner.as_ref().clone();
                self.define_loop_vars_from_elem(target, &elem_ty, wrap_in_ref);
            }
            Type::Dict(k, v) => {
                // For .items() iteration
                if target.contains(',') {
                    let targets: Vec<_> = target.split(',').map(|s| s.trim()).collect();
                    if targets.len() == 2 {
                        self.scope.define(targets[0], Type::Ref(k.clone()), false);
                        self.scope.define(targets[1], Type::Ref(v.clone()), false);
                    }
                }
            }
            Type::Tuple(elems) => {
                if target.contains(',') {
                    let targets: Vec<_> = target.split(',').map(|s| s.trim()).collect();
                    for (t, ty) in targets.iter().zip(elems.iter()) {
                        let final_ty = if wrap_in_ref && !matches!(ty, Type::Ref(_)) {
                            Type::Ref(Box::new(ty.clone()))
                        } else {
                            ty.clone()
                        };
                        self.scope.define(t, final_ty, false);
                    }
                }
            }
            _ => {
                // Unknown or Range type
                if target.contains(',') {
                    for t in target.split(',') {
                        self.scope.define(t.trim(), Type::Unknown, false);
                    }
                } else {
                    let ty = if wrap_in_ref {
                        Type::Ref(Box::new(Type::Int))
                    } else {
                        Type::Int
                    };
                    self.scope.define(target, ty, false);
                }
            }
        }
    }

    /// Helper to define loop variables from element type (handles tuple unpacking)
    fn define_loop_vars_from_elem(&mut self, target: &str, elem_ty: &Type, wrap_in_ref: bool) {
        if target.contains(',') {
            // Tuple unpacking: (k, v) for t in list_of_tuples
            if let Type::Tuple(elems) = elem_ty {
                let targets: Vec<_> = target.split(',').map(|s| s.trim()).collect();
                for (t, ty) in targets.iter().zip(elems.iter()) {
                    let final_ty = if wrap_in_ref && !matches!(ty, Type::Ref(_)) {
                        Type::Ref(Box::new(ty.clone()))
                    } else {
                        ty.clone()
                    };
                    self.scope.define(t, final_ty, false);
                }
            } else {
                // Fallback for non-tuple
                for t in target.split(',') {
                    self.scope.define(t.trim(), Type::Unknown, false);
                }
            }
        } else {
            // Single variable
            let final_ty = if wrap_in_ref && !matches!(elem_ty, Type::Ref(_)) {
                Type::Ref(Box::new(elem_ty.clone()))
            } else {
                elem_ty.clone()
            };
            self.scope.define(target, final_ty, false);
        }
    }

    /// Handle special method calls that require transformation
    /// Returns Some(IrExpr) if the method was handled specially, None otherwise
    fn try_handle_special_method(
        &mut self,
        target_ir: &IrExpr,
        _target_ty: &Type,
        method: &str,
        args: &[Expr],
    ) -> Result<Option<IrExpr>, TsuchinokoError> {
        match method {
            "items" if args.is_empty() => {
                // dict.items() -> dict.iter().map(|(k,v)|(*k, v.clone())) (only for Dict types)
                // For user-defined structs with items() method, keep the call as-is
                // Explicitly handle both Dict and non-Dict cases
                match _target_ty {
                    Type::Dict(_, _) => {
                        // dict.iter() returns (&K, &V), we need owned (K, V) for filter/map
                        // Use iter().map() to clone values for ownership
                        let iter_call = IrExpr::MethodCall {
                            target: Box::new(target_ir.clone()),
                            method: "iter".to_string(),
                            args: vec![],
                        };
                        Ok(Some(IrExpr::MethodCall {
                            target: Box::new(iter_call),
                            method: "map".to_string(),
                            args: vec![IrExpr::RawCode("|(k, v)| (*k, v.clone())".to_string())],
                        }))
                    }
                    Type::Struct(_) => {
                        // User-defined struct with items() method - keep as-is
                        Ok(None)
                    }
                    _ => {
                        // Unknown or other type - don't transform
                        Ok(None)
                    }
                }
            }
            "join" if args.len() == 1 => {
                // "sep".join(iterable) -> iterable.iter().map(|x| x.to_string()).collect().join(&sep)
                let iterable_ast = &args[0];
                let iterable_ty = self.infer_type(iterable_ast);
                let iterable_ir = self.analyze_expr(iterable_ast)?;

                let needs_string_conversion = match &iterable_ty {
                    Type::List(inner) => **inner != Type::String,
                    _ => true,
                };

                if needs_string_conversion {
                    let iter_call = IrExpr::MethodCall {
                        target: Box::new(iterable_ir),
                        method: "iter".to_string(),
                        args: vec![],
                    };
                    let closure = IrExpr::Closure {
                        params: vec!["x".to_string()],
                        body: vec![IrNode::Expr(IrExpr::MethodCall {
                            target: Box::new(IrExpr::Var("x".to_string())),
                            method: "to_string".to_string(),
                            args: vec![],
                        })],
                        ret_type: Type::String,
                    };
                    let map_call = IrExpr::MethodCall {
                        target: Box::new(iter_call),
                        method: "map".to_string(),
                        args: vec![closure],
                    };
                    let collect_call = IrExpr::MethodCall {
                        target: Box::new(map_call),
                        method: "collect::<Vec<String>>".to_string(),
                        args: vec![],
                    };
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(collect_call),
                        method: "join".to_string(),
                        args: vec![target_ir.clone()],
                    }))
                } else {
                    Ok(Some(IrExpr::MethodCall {
                        target: Box::new(iterable_ir),
                        method: "join".to_string(),
                        args: vec![target_ir.clone()],
                    }))
                }
            }
            _ => Ok(None), // Not a special method, use default handling
        }
    }

    /// Get expected parameter types for built-in methods
    fn get_method_param_types(&self, target_ty: &Type, method: &str) -> Vec<Type> {
        match (target_ty, method) {
            (Type::List(inner), "push") | (Type::List(inner), "append") => {
                vec![inner.as_ref().clone()]
            }
            (Type::List(inner), "extend") => {
                vec![Type::Ref(Box::new(Type::List(inner.clone())))]
            }
            _ => vec![],
        }
    }

    fn to_snake_case(&self, s: &str) -> String {
        let mut res = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                res.push('_');
            }
            res.push(c.to_lowercase().next().unwrap());
        }
        res
    }

    fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Struct(name) => {
                if let Some(info) = self.scope.lookup(name) {
                    // Prevent infinite recursion: if the resolved type is the same Struct, return it
                    if let Type::Struct(resolved_name) = &info.ty {
                        if resolved_name == name {
                            return ty.clone();
                        }
                    }
                    return self.resolve_type(&info.ty);
                }
                // Return the Struct type itself if not found (it's a legitimate struct type)
                ty.clone()
            }
            Type::Ref(inner) => self.resolve_type(inner),
            _ => ty.clone(),
        }
    }

    /// Handle built-in function calls (range, len, list, str, tuple, dict, max)
    /// Returns Some(IrExpr) if handled, None if not a built-in
    fn try_handle_builtin_call(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<Option<IrExpr>, TsuchinokoError> {
        match (name, args.len()) {
            ("range", 1) => {
                let start = IrExpr::IntLit(0);
                let end = self.analyze_expr(&args[0])?;
                Ok(Some(IrExpr::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                }))
            }
            ("range", 2) => {
                let start = self.analyze_expr(&args[0])?;
                let end = self.analyze_expr(&args[1])?;
                Ok(Some(IrExpr::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                }))
            }
            ("len", 1) => {
                let arg = self.analyze_expr(&args[0])?;
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(arg),
                    method: "len".to_string(),
                    args: vec![],
                }))
            }
            ("list", 1) => {
                // list(iterable) -> iterable.collect::<Vec<_>>()
                // Special case: list(dict.items()) needs iter().map(|(k,v)| (*k, v.clone())).collect()

                // Check if arg is a method call to .items() on a dict
                if let Expr::Call {
                    func,
                    args: call_args,
                    ..
                } = &args[0]
                {
                    if call_args.is_empty() {
                        if let Expr::Attribute {
                            value: item_target,
                            attr,
                        } = func.as_ref()
                        {
                            if attr == "items" {
                                let target_ty = self.infer_type(item_target);
                                if matches!(target_ty, Type::Dict(_, _)) {
                                    // This is list(dict.items()) - generate iter().map(clone).collect()
                                    let ir_target = self.analyze_expr(item_target)?;
                                    let iter_call = IrExpr::MethodCall {
                                        target: Box::new(ir_target),
                                        method: "iter".to_string(),
                                        args: vec![],
                                    };
                                    let map_call = IrExpr::MethodCall {
                                        target: Box::new(iter_call),
                                        method: "map".to_string(),
                                        args: vec![IrExpr::RawCode(
                                            "|(k, v)| (*k, v.clone())".to_string(),
                                        )],
                                    };
                                    return Ok(Some(IrExpr::MethodCall {
                                        target: Box::new(map_call),
                                        method: "collect::<Vec<_>>".to_string(),
                                        args: vec![],
                                    }));
                                }
                            }
                        }
                    }
                }

                let arg = self.analyze_expr(&args[0])?;

                // If arg is a MethodCall (likely an iterator like .iter())
                // or a GenExpr/ListComp, always add collect()
                if matches!(arg, IrExpr::MethodCall { .. } | IrExpr::ListComp { .. }) {
                    return Ok(Some(IrExpr::MethodCall {
                        target: Box::new(arg),
                        method: "collect::<Vec<_>>".to_string(),
                        args: vec![],
                    }));
                }
                // Otherwise, use .to_vec() to convert slice/vec to owned Vec
                // This handles list(some_slice) -> some_slice.to_vec()
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(arg),
                    method: "to_vec".to_string(),
                    args: vec![],
                }))
            }
            ("str", 1) => {
                let arg = self.analyze_expr(&args[0])?;
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(arg),
                    method: "to_string".to_string(),
                    args: vec![],
                }))
            }
            ("tuple", 1) => {
                let ir_arg = self.analyze_expr(&args[0])?;
                // If already an iterator/collection producing IR, don't wrap
                if matches!(ir_arg, IrExpr::ListComp { .. } | IrExpr::MethodCall { .. }) {
                    return Ok(Some(ir_arg));
                }
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(ir_arg),
                    method: "collect::<Vec<_>>".to_string(),
                    args: vec![],
                }))
            }
            ("dict", 1) => {
                // dict(x) -> x.clone()
                // Python dict(some_dict) creates a copy, Rust .clone() does the same
                let ir_arg = self.analyze_expr(&args[0])?;
                Ok(Some(IrExpr::MethodCall {
                    target: Box::new(ir_arg),
                    method: "clone".to_string(),
                    args: vec![],
                }))
            }
            ("max", 1) => {
                let arg = self.analyze_expr(&args[0])?;
                let iter_call = IrExpr::MethodCall {
                    target: Box::new(arg),
                    method: "iter".to_string(),
                    args: vec![],
                };
                let max_call = IrExpr::MethodCall {
                    target: Box::new(iter_call),
                    method: "max".to_string(),
                    args: vec![],
                };
                let copied_call = IrExpr::MethodCall {
                    target: Box::new(max_call),
                    method: "cloned".to_string(),
                    args: vec![],
                };
                let unwrap_call = IrExpr::MethodCall {
                    target: Box::new(copied_call),
                    method: "unwrap".to_string(),
                    args: vec![],
                };
                Ok(Some(unwrap_call))
            }
            _ => Ok(None),
        }
    }

    fn get_func_name_for_debug(&self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(name) => name.clone(),
            Expr::Attribute { attr, .. } => attr.clone(),
            _ => "complex_call".to_string(),
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

        if let IrNode::FuncDecl {
            name,
            params,
            ret,
            body,
        } = &ir[0]
        {
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
}
