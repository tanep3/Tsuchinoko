//! Semantic analysis module
//!
//! 意味解析を行うモジュール。AST から IR への変換を担当する。
//!
//! ## サブモジュール
//! - `types` - 型定義
//! - `scope` - スコープ管理
//! - `type_infer` - 型推論（トレイト定義）
//! - `operators` - 演算子変換
//! - `coercion` - 型変換・強制
//! - `builtins` - 組み込み関数
//!
//! ## 分割モジュール（SemanticAnalyzer impl）
//! - `analyze_statements` - 文の解析 (analyze_stmt)
//! - `analyze_expressions` - 式の解析 (analyze_expr)
//! - `analyze_calls` - 関数呼び出し処理
//! - `analyze_types` - 型推論実装 (infer_type)

mod analyze_calls;
mod analyze_expressions;
mod analyze_statements;
mod analyze_types;
pub mod builtins;
pub mod coercion;
pub mod operators;
mod scope;
pub mod type_infer;
mod types;

pub use operators::convert_binop;
pub use scope::*;
pub use type_infer::TypeInference;
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
    /// Function name -> Vec of (param_name, param_type, default_expr, is_variadic) for default arg handling
    #[allow(clippy::type_complexity)]
    func_param_info: std::collections::HashMap<String, Vec<(String, Type, Option<Expr>, bool)>>,
    /// External imports: (module, alias) - e.g., ("numpy", "np")
    external_imports: Vec<(String, String)>,
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
            func_param_info: std::collections::HashMap::new(),
            external_imports: Vec::new(),
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
                | IrNode::PyO3Import { .. }
                | IrNode::Sequence(_) => {
                    other_decls.push(node);
                }
                _ => {
                    main_body.push(node);
                }
            }
        }

        if main_body.is_empty() {
            // Check if we have a standalone def main() that should be our entry point
            if let Some(pos) = other_decls
                .iter()
                .position(|n| matches!(n, IrNode::FuncDecl { name, .. } if name == "main"))
            {
                if let IrNode::FuncDecl {
                    name: _,
                    params,
                    ret,
                    body,
                } = other_decls.remove(pos)
                {
                    other_decls.push(IrNode::FuncDecl {
                        name: "__top_level__".to_string(),
                        params,
                        ret,
                        body,
                    });
                }
            }
        } else {
            other_decls.push(IrNode::FuncDecl {
                name: "__top_level__".to_string(),
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
                                | "add" | "discard"  // V1.5.0: Set methods
                                | "update"  // V1.5.0: Dict methods
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
            // V1.3.0: Check for TupleAssign (x, y = y, x + y)
            Stmt::TupleAssign { targets, .. } => {
                for target in targets {
                    // Only mark as mutable if this variable was already declared
                    // (if it's in seen_vars, it means it was declared earlier)
                    if seen_vars.contains(target) {
                        reassigned_vars.insert(target.clone());
                    }
                    // Always add to seen_vars for future reference
                    seen_vars.insert(target.clone());
                }
            }
            // V1.5.0: Recurse into TryExcept bodies to detect mutations
            Stmt::TryExcept {
                try_body,
                except_clauses,
                finally_body,
            } => {
                for s in try_body {
                    self.collect_mutations(s, reassigned_vars, mutated_vars, seen_vars);
                }
                for clause in except_clauses {
                    for s in &clause.body {
                        self.collect_mutations(s, reassigned_vars, mutated_vars, seen_vars);
                    }
                }
                if let Some(fb) = finally_body {
                    for s in fb {
                        self.collect_mutations(s, reassigned_vars, mutated_vars, seen_vars);
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

        // V1.5.0: Add collected mutations to mutable_vars for this scope
        self.mutable_vars.extend(reassigned_vars.clone());
        self.mutable_vars.extend(mutated_vars.clone());

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

            // Check for early return narrowing AFTER processing the if statement
            // `if x is None: return ...` - after this, x is guaranteed to be non-None
            if let Stmt::If {
                condition,
                then_body,
                elif_clauses,
                else_body,
            } = stmt
            {
                if else_body.is_none() && elif_clauses.is_empty() {
                    // Check if then_body contains only a return statement
                    let is_early_return =
                        then_body.len() == 1 && matches!(&then_body[0], Stmt::Return { .. });

                    if is_early_return {
                        if let Some((var_name, is_none_check)) = self.extract_none_check(condition)
                        {
                            if is_none_check {
                                // `if x is None: return ...` pattern detected
                                // After this statement, x is guaranteed to be non-None
                                // Apply narrowing to subsequent statements
                                if let Some(var_info) = self.scope.lookup(&var_name) {
                                    if let Type::Optional(inner) = &var_info.ty {
                                        // Narrow x to T for the rest of this block
                                        self.scope.narrow_type(&var_name, *inner.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
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

                // If type hint is concrete (String, Int, etc.) but expression is Type::Any,
                // wrap with JsonConversion for proper type conversion
                let expr_ty = self.infer_type(value);
                let ir_value =
                    if matches!(expr_ty, Type::Any) && !matches!(ty, Type::Any | Type::Unknown) {
                        let conversion = match &ty {
                            Type::Float => Some("f64"),
                            Type::Int => Some("i64"),
                            Type::String => Some("String"),
                            Type::Bool => Some("bool"),
                            _ => None,
                        };
                        if let Some(conv) = conversion {
                            IrExpr::JsonConversion {
                                target: Box::new(ir_value),
                                convert_to: conv.to_string(),
                            }
                        } else {
                            ir_value
                        }
                    } else {
                        ir_value
                    };

                // V1.3.0: If type hint is List with known element type, update IrExpr::List's elem_type
                // This ensures emitter can correctly add .to_string() for String elements in tuples
                let ir_value = if let Type::List(elem_ty) = &ty {
                    if let IrExpr::List {
                        elem_type: _,
                        elements,
                    } = ir_value
                    {
                        IrExpr::List {
                            elem_type: *elem_ty.clone(),
                            elements,
                        }
                    } else {
                        ir_value
                    }
                } else {
                    ir_value
                };

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
            // V1.3.0: Handle TupleAssign with mutability info
            Stmt::TupleAssign {
                targets,
                value,
                starred_index,
            } => {
                // Delegate star unpacking to regular analyze_stmt
                if starred_index.is_some() {
                    return self.analyze_stmt(stmt);
                }

                let ir_value = self.analyze_expr(value)?;
                let is_decl = self.scope.lookup(&targets[0]).is_none();

                // Check if value is a List - need special handling
                let result_type = self.infer_type(value);
                let is_list = matches!(&result_type, Type::List(_))
                    || matches!(&result_type, Type::Ref(inner) if matches!(inner.as_ref(), Type::List(_)));

                if is_decl {
                    let elem_types = if let Type::Tuple(types) = &result_type {
                        if types.len() == targets.len() {
                            types.clone()
                        } else {
                            vec![Type::Unknown; targets.len()]
                        }
                    } else if let Type::List(elem) = &result_type {
                        vec![*elem.clone(); targets.len()]
                    } else {
                        vec![Type::Unknown; targets.len()]
                    };

                    let mut decl_targets = Vec::new();
                    for (i, target) in targets.iter().enumerate() {
                        let ty = elem_types.get(i).unwrap_or(&Type::Unknown).clone();
                        // Check if this target will be reassigned later
                        let is_mutable = reassigned_vars.contains(target);
                        self.scope.define(target, ty.clone(), is_mutable);
                        decl_targets.push((target.clone(), ty, is_mutable));
                    }

                    // If value is a List, convert to tuple of indexed accesses
                    let final_value = if is_list {
                        let indices: Vec<IrExpr> = (0..targets.len())
                            .map(|i| IrExpr::Index {
                                target: Box::new(ir_value.clone()),
                                index: Box::new(IrExpr::Cast {
                                    target: Box::new(IrExpr::IntLit(i as i64)),
                                    ty: "usize".to_string(),
                                }),
                            })
                            .collect();
                        IrExpr::Tuple(indices)
                    } else {
                        ir_value
                    };

                    Ok(IrNode::MultiVarDecl {
                        targets: decl_targets,
                        value: Box::new(final_value),
                    })
                } else {
                    Ok(IrNode::MultiAssign {
                        targets: targets.clone(),
                        value: Box::new(ir_value),
                    })
                }
            }
            // For other statements, delegate to the regular analyze_stmt
            _ => self.analyze_stmt(stmt),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn type_from_hint(&self, hint: &TypeHint) -> Type {
        let params: Vec<Type> = hint.params.iter().map(|h| self.type_from_hint(h)).collect();

        Type::from_python_hint(&hint.name, &params)
    }

    fn extract_none_check(&self, condition: &Expr) -> Option<(String, bool)> {
        match condition {
            Expr::BinOp { left, op, right } => {
                match op {
                    AstBinOp::Is => {
                        // x is None
                        if let (Expr::Ident(var), Expr::NoneLiteral) =
                            (left.as_ref(), right.as_ref())
                        {
                            return Some((var.clone(), true));
                        }
                    }
                    AstBinOp::IsNot => {
                        // x is not None
                        if let (Expr::Ident(var), Expr::NoneLiteral) =
                            (left.as_ref(), right.as_ref())
                        {
                            return Some((var.clone(), false));
                        }
                    }
                    _ => {}
                }
            }
            Expr::UnaryOp {
                op: AstUnaryOp::Not,
                operand,
            } => {
                // not (x is None) => equivalent to x is not None
                if let Some((var, is_none)) = self.extract_none_check(operand) {
                    return Some((var, !is_none));
                }
            }
            _ => {}
        }
        None
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
            AstBinOp::NotIn => IrBinOp::NotContains, // V1.3.0
            AstBinOp::Is => IrBinOp::Is,
            AstBinOp::IsNot => IrBinOp::IsNot,
            // Bitwise operators (V1.3.0)
            AstBinOp::BitAnd => IrBinOp::BitAnd,
            AstBinOp::BitOr => IrBinOp::BitOr,
            AstBinOp::BitXor => IrBinOp::BitXor,
            AstBinOp::Shl => IrBinOp::Shl,
            AstBinOp::Shr => IrBinOp::Shr,
            AstBinOp::MatMul => IrBinOp::MatMul, // V1.3.0
        }
    }
}

#[cfg(test)]
mod tests;
