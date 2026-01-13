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
pub mod emit_plan;
pub mod lowering;  // V1.7.0
pub mod operators;
mod scope;
pub mod type_infer;
mod types;

pub use operators::convert_binop;
pub use scope::*;
pub use type_infer::TypeInference;
pub use types::*;
pub use emit_plan::{build_emit_plan, EmitPlan, FuncEmitPlan};

use crate::error::TsuchinokoError;
use crate::ir::{HoistedVar, IrAugAssignOp, IrBinOp, IrExpr, IrExprKind, IrNode, IrUnaryOp};
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
    /// V1.5.2: Struct name -> Vec of (field_name, default_value_ir) for constructor default values
    struct_field_defaults: std::collections::HashMap<String, Vec<(String, IrExpr)>>,
    /// Variables that need to be mutable (targets of AugAssign or reassignment)
    mutable_vars: std::collections::HashSet<String>,
    /// Function name -> Vec of (param_name, param_type, default_expr, is_variadic) for default arg handling
    #[allow(clippy::type_complexity)]
    func_param_info: std::collections::HashMap<String, Vec<(String, Type, Option<Expr>, bool)>>,
    /// External imports: (module, alias) - e.g., ("numpy", "np")
    external_imports: Vec<(String, String)>,
    /// Variables that need hoisting: (name, type, defined_depth, used_depth)
    /// Collected during analysis, variables where used_depth < defined_depth need hoisting
    hoisted_var_candidates: std::collections::HashMap<String, (Type, usize, usize)>,
    /// Current function's base scope depth (for relative depth calculation)
    func_base_depth: usize,
    /// V1.5.2: Current function may raise an exception (Result化が必要)
    current_func_may_raise: bool,
    /// V1.5.2: Functions that may raise (for callee_may_raise detection)
    may_raise_funcs: std::collections::HashSet<String>,
    /// V1.6.0: Struct name -> parent class name (for inheritance/composition)
    struct_bases: std::collections::HashMap<String, String>,
    /// V1.6.0: Current class being analyzed (for self.field -> self.base.field)
    current_class_base: Option<String>,
    /// V1.6.0 FT-005: Types checked by isinstance (for DynamicValue enum generation)
    isinstance_types: Vec<Type>,
    /// V1.7.0: Aliases that refer to Modules or Items (alias -> full_target)
    module_global_aliases: std::collections::HashMap<String, String>,
    /// V1.7.0: Current function needs PythonBridge argument
    current_func_needs_bridge: bool,
    /// V1.7.0: Functions that need PythonBridge (for callee_needs_bridge detection)
    needs_bridge_funcs: std::collections::HashSet<String>,
    /// V1.7.0: Expression ID counter
    expr_id_counter: u32,
    /// V1.7.0: Type Table (ExprId -> Type)
    type_table: std::collections::HashMap<crate::ir::ExprId, Type>,
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
            struct_field_defaults: std::collections::HashMap::new(),
            mutable_vars: std::collections::HashSet::new(),
            func_param_info: std::collections::HashMap::new(),
            external_imports: Vec::new(),
            hoisted_var_candidates: std::collections::HashMap::new(),
            func_base_depth: 0,
            current_func_may_raise: false,
            may_raise_funcs: std::collections::HashSet::new(),
            struct_bases: std::collections::HashMap::new(),
            current_class_base: None,
            isinstance_types: Vec::new(),
            // temp_counter: 0, // Removed as unused
            module_global_aliases: std::collections::HashMap::new(),
            current_func_needs_bridge: false,
            needs_bridge_funcs: std::collections::HashSet::new(),
            expr_id_counter: 0,
            type_table: std::collections::HashMap::new(),
        }
    }
    
    /// V1.7.0: Generate next ExprId
    fn next_expr_id(&mut self) -> crate::ir::ExprId {
        let id = crate::ir::ExprId(self.expr_id_counter);
        self.expr_id_counter += 1;
        id
    }
    
    /// V1.7.0: Set type for an expression
    fn set_type(&mut self, id: crate::ir::ExprId, ty: Type) {
        self.type_table.insert(id, ty);
    }
    
    /// V1.7.0: Create a new IrExpr with ID and record its type
    fn create_expr(&mut self, kind: crate::ir::IrExprKind, ty: Type) -> crate::ir::IrExpr {
        let id = self.next_expr_id();
        self.set_type(id, ty.clone());
        crate::ir::IrExpr { id, kind }
    }

    /// V1.5.2 (2-Pass): Quick check if a function body may raise exceptions
    /// This is a lightweight check on AST only, without full semantic analysis.
    fn quick_may_raise_check(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.stmt_may_raise(stmt) {
                return true;
            }
        }
        false
    }

    /// V1.5.2 (2-Pass): Check if a statement may raise
    fn stmt_may_raise(&self, stmt: &Stmt) -> bool {
        match stmt {
            // Explicit raise statement
            Stmt::Raise { .. } => true,

            // Expression statement - check the expression
            Stmt::Expr(expr) => self.expr_may_raise(expr),

            // Assignment - check the value expression
            Stmt::Assign { value, .. } => self.expr_may_raise(value),

            // Augmented assignment - check the value
            Stmt::AugAssign { value, .. } => self.expr_may_raise(value),

            // Return - check the return value
            Stmt::Return(Some(expr)) => self.expr_may_raise(expr),

            // If statement - check all branches
            Stmt::If {
                then_body,
                elif_clauses,
                else_body,
                ..
            } => {
                if self.quick_may_raise_check(then_body) {
                    return true;
                }
                for (_, elif_body) in elif_clauses {
                    if self.quick_may_raise_check(elif_body) {
                        return true;
                    }
                }
                if let Some(else_b) = else_body {
                    if self.quick_may_raise_check(else_b) {
                        return true;
                    }
                }
                false
            }

            // For loop - check body
            Stmt::For { body, .. } => self.quick_may_raise_check(body),

            // While loop - check body
            Stmt::While { body, .. } => self.quick_may_raise_check(body),

            // Try - if it has try, it's handling exceptions, the body may raise
            Stmt::TryExcept { try_body, .. } => self.quick_may_raise_check(try_body),

            // Default - doesn't raise
            _ => false,
        }
    }

    /// V1.5.2 (2-Pass): Check if an expression may raise
    fn expr_may_raise(&self, expr: &Expr) -> bool {
        match expr {
            // PyO3 call: module.func(...)
            Expr::Call { func, .. } => {
                // Check if it's a PyO3 module call (module.func())
                if let Expr::Attribute { value, .. } = func.as_ref() {
                    if let Expr::Ident(module) = value.as_ref() {
                        if self
                            .external_imports
                            .iter()
                            .any(|(_, alias)| alias == module)
                        {
                            return true;
                        }
                    }
                }

                // Check if it's a from-import call (func())
                if let Expr::Ident(name) = func.as_ref() {
                    if self.external_imports.iter().any(|(_, item)| item == name) {
                        return true;
                    }
                }

                // Check args for any raising expressions
                if let Expr::Call { args, .. } = expr {
                    return args.iter().any(|a| self.expr_may_raise(a));
                }

                false
            }

            // Binary operation - check both sides
            Expr::BinOp { left, right, .. } => {
                self.expr_may_raise(left) || self.expr_may_raise(right)
            }

            // Attribute access - check target
            Expr::Attribute { value, .. } => self.expr_may_raise(value),

            // Index access - check target and index
            Expr::Index { target, index } => {
                self.expr_may_raise(target) || self.expr_may_raise(index)
            }

            // Default - doesn't raise
            _ => false,
        }
    }

    /// V1.5.2 (2-Pass Step 3): Check if function body calls any may_raise function
    fn body_calls_may_raise_func(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.stmt_calls_may_raise_func(stmt) {
                return true;
            }
        }
        false
    }

    /// V1.5.2 (2-Pass Step 3): Check if a statement calls any may_raise function
    fn stmt_calls_may_raise_func(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(expr) => self.expr_calls_may_raise_func(expr),
            Stmt::Assign { value, .. } => self.expr_calls_may_raise_func(value),
            Stmt::AugAssign { value, .. } => self.expr_calls_may_raise_func(value),
            Stmt::Return(Some(expr)) => self.expr_calls_may_raise_func(expr),
            Stmt::If {
                then_body,
                elif_clauses,
                else_body,
                ..
            } => {
                if self.body_calls_may_raise_func(then_body) {
                    return true;
                }
                for (_, elif_body) in elif_clauses {
                    if self.body_calls_may_raise_func(elif_body) {
                        return true;
                    }
                }
                if let Some(else_b) = else_body {
                    if self.body_calls_may_raise_func(else_b) {
                        return true;
                    }
                }
                false
            }
            Stmt::For { body, .. } => self.body_calls_may_raise_func(body),
            Stmt::While { body, .. } => self.body_calls_may_raise_func(body),
            Stmt::TryExcept { try_body, .. } => self.body_calls_may_raise_func(try_body),
            _ => false,
        }
    }

    /// V1.5.2 (2-Pass Step 3): Check if an expression calls any may_raise function
    fn expr_calls_may_raise_func(&self, expr: &Expr) -> bool {
        match expr {
            // Function call - check if callee is may_raise
            Expr::Call { func, args, .. } => {
                // Check if calling a known may_raise function
                if let Expr::Ident(name) = func.as_ref() {
                    if let Some(var_info) = self.scope.lookup(name) {
                        if let Type::Func {
                            may_raise: true, ..
                        } = &var_info.ty
                        {
                            return true;
                        }
                    }
                    // V1.4.0 Phase G: Check if this is an external import call (e.g. numpy.mean)
                    // Bridge calls are always considered may_raise in Tsuchinoko
                    if self.module_global_aliases.contains_key(name) {
                        return true;
                    }
                }
                // Check args recursively
                args.iter().any(|a| self.expr_calls_may_raise_func(a))
            }
            Expr::BinOp { left, right, .. } => {
                self.expr_calls_may_raise_func(left) || self.expr_calls_may_raise_func(right)
            }
            Expr::Attribute { value, .. } => self.expr_calls_may_raise_func(value),
            _ => false,
        }
    }

    /// V1.5.2: Pre-process import statements to populate external_imports
    ///
    /// This must be done BEFORE forward_declare_functions so that may_raise
    /// can correctly detect external library calls in function bodies.
    fn preprocess_imports(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            if let Stmt::Import {
                module,
                alias,
                items,
            } = stmt
            {
                if let Some(ref item_list) = items {
                    // "from module import a, b, c"
                    for item in item_list {
                        self.module_global_aliases
                            .insert(item.clone(), format!("{module}.{item}"));
                        if !crate::bridge::module_table::is_native_module(module) {
                            self.external_imports.push((module.clone(), item.clone()));
                        }
                    }
                } else {
                    // "import module" or "import module as alias"
                    let effective_name = alias.as_ref().unwrap_or(module);
                    self.module_global_aliases
                        .insert(effective_name.clone(), module.clone());
                    if !crate::bridge::module_table::is_native_module(module) {
                        self.external_imports
                            .push((module.clone(), effective_name.clone()));
                    }
                }
            }
        }
    }

    pub fn define(&mut self, name: &str, ty: Type, mutable: bool) {
        self.scope.define(name, ty, mutable);
    }

    /// V1.5.2: Forward-declare all function signatures
    ///
    /// This allows top-level code to correctly infer types for function calls
    /// to functions defined later in the file.
    fn forward_declare_functions(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            if let Stmt::FuncDef {
                name,
                params,
                return_type,
                body,
                ..
            } = stmt
            {
                let ret_type = return_type
                    .as_ref()
                    .map(|th| self.type_from_hint(th))
                    .unwrap_or(Type::Unit);

                // Collect parameter types
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        let hinted_ty = p.type_hint.as_ref().map(|th| self.type_from_hint(th));
                        let base_ty = if matches!(hinted_ty, Some(Type::Optional(_))) {
                            hinted_ty.unwrap()
                        } else {
                            p.type_hint
                                .as_ref()
                                .map(|th| self.type_from_hint(th))
                                .unwrap_or(Type::Unknown)
                        };
                        let base_ty = if matches!(
                            (&p.default, &base_ty),
                            (Some(Expr::NoneLiteral), Type::Optional(_))
                        ) {
                            base_ty
                        } else if matches!(p.default, Some(Expr::NoneLiteral)) {
                            Type::Optional(Box::new(base_ty))
                        } else {
                            base_ty
                        };
                        if p.variadic {
                            Type::List(Box::new(base_ty))
                        } else {
                            base_ty
                        }
                    })
                    .collect();

                // V1.5.2 (2-Pass): Determine may_raise from function body
                let may_raise = self.quick_may_raise_check(body);

                // Register function in scope
                self.scope.define(
                    name,
                    Type::Func {
                        params: param_types,
                        ret: Box::new(ret_type),
                        is_boxed: false,
                        may_raise,
                    },
                    false,
                );
            }
        }

        // V1.5.2 (2-Pass Step 3): Propagate may_raise through call chains
        // Functions that call may_raise functions should also be may_raise
        // We iterate until no changes are made (usually 2 iterations max)
        loop {
            let mut changed = false;

            for stmt in stmts {
                if let Stmt::FuncDef { name, body, .. } = stmt {
                    // Check if this function calls any may_raise function
                    let calls_may_raise = self.body_calls_may_raise_func(body);

                    // Get current may_raise status from scope
                    if let Some(var_info) = self.scope.lookup(name) {
                        if let Type::Func {
                            may_raise,
                            params,
                            ret,
                            is_boxed,
                        } = &var_info.ty
                        {
                            if !may_raise && calls_may_raise {
                                // Update to may_raise = true
                                self.scope.define(
                                    name,
                                    Type::Func {
                                        params: params.clone(),
                                        ret: ret.clone(),
                                        is_boxed: *is_boxed,
                                        may_raise: true,
                                    },
                                    false,
                                );
                                changed = true;
                            }
                        }
                    }
                }
            }

            if !changed {
                break;
            }
        }

        // V1.5.2 (2-Pass Step 4): Refine Unknown parameter types from call sites
        // For parameters with Type::List(Unknown), infer element type from caller's arguments
        self.refine_unknown_param_types(stmts);
    }

    /// V1.5.2: Refine Unknown types in function parameters by analyzing call sites
    ///
    /// When a function has `nums: list` (without element type), we infer the element
    /// type from how the function is called, e.g., `func([1, 2, 3])` -> List<Int>
    fn refine_unknown_param_types(&mut self, stmts: &[Stmt]) {
        // Collect call site argument types for each function
        let mut call_arg_types: std::collections::HashMap<String, Vec<Vec<Type>>> =
            std::collections::HashMap::new();

        // Walk all statements to find function calls
        for stmt in stmts {
            self.collect_call_arg_types_from_stmt(stmt, &mut call_arg_types);
        }

        // Update function signatures based on collected info
        for (func_name, call_args_list) in call_arg_types {
            if let Some(var_info) = self.scope.lookup(&func_name) {
                if let Type::Func {
                    params,
                    ret,
                    is_boxed,
                    may_raise,
                } = &var_info.ty
                {
                    let mut refined_params = params.clone();
                    let mut changed = false;

                    // For each parameter position, check if we can refine
                    for (i, param_type) in params.iter().enumerate() {
                        if let Some(refined) = self.try_refine_type(param_type, &call_args_list, i)
                        {
                            refined_params[i] = refined;
                            changed = true;
                        }
                    }

                    if changed {
                        self.scope.define(
                            &func_name,
                            Type::Func {
                                params: refined_params,
                                ret: ret.clone(),
                                is_boxed: *is_boxed,
                                may_raise: *may_raise,
                            },
                            false,
                        );
                    }
                }
            }
        }
    }

    /// Collect argument types from all call sites in a statement
    fn collect_call_arg_types_from_stmt(
        &self,
        stmt: &Stmt,
        result: &mut std::collections::HashMap<String, Vec<Vec<Type>>>,
    ) {
        match stmt {
            Stmt::FuncDef { body, .. } => {
                for s in body {
                    self.collect_call_arg_types_from_stmt(s, result);
                }
            }
            Stmt::Assign { value, .. } | Stmt::Return(Some(value)) => {
                self.collect_call_arg_types_from_expr(value, result);
            }
            Stmt::Expr(expr) => {
                self.collect_call_arg_types_from_expr(expr, result);
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.collect_call_arg_types_from_expr(condition, result);
                for s in then_body {
                    self.collect_call_arg_types_from_stmt(s, result);
                }
                if let Some(eb) = else_body {
                    for s in eb {
                        self.collect_call_arg_types_from_stmt(s, result);
                    }
                }
            }
            Stmt::For { iter, body, .. } => {
                self.collect_call_arg_types_from_expr(iter, result);
                for s in body {
                    self.collect_call_arg_types_from_stmt(s, result);
                }
            }
            Stmt::While { condition, body } => {
                self.collect_call_arg_types_from_expr(condition, result);
                for s in body {
                    self.collect_call_arg_types_from_stmt(s, result);
                }
            }
            _ => {}
        }
    }

    /// Collect argument types from call expressions
    fn collect_call_arg_types_from_expr(
        &self,
        expr: &Expr,
        result: &mut std::collections::HashMap<String, Vec<Vec<Type>>>,
    ) {
        match expr {
            Expr::Call { func, args, .. } => {
                if let Expr::Ident(func_name) = func.as_ref() {
                    let arg_types: Vec<Type> = args.iter().map(|a| self.infer_type(a)).collect();
                    result.entry(func_name.clone()).or_default().push(arg_types);
                }
                // Recurse into args
                for arg in args {
                    self.collect_call_arg_types_from_expr(arg, result);
                }
            }
            Expr::BinOp { left, right, .. } => {
                self.collect_call_arg_types_from_expr(left, result);
                self.collect_call_arg_types_from_expr(right, result);
            }
            Expr::Attribute { value, .. } => {
                self.collect_call_arg_types_from_expr(value, result);
            }
            Expr::Index { target, index } => {
                self.collect_call_arg_types_from_expr(target, result);
                self.collect_call_arg_types_from_expr(index, result);
            }
            Expr::List(elements) | Expr::Tuple(elements) => {
                for e in elements {
                    self.collect_call_arg_types_from_expr(e, result);
                }
            }
            _ => {}
        }
    }

    /// Try to refine a type if it contains Unknown
    fn try_refine_type(
        &self,
        param_type: &Type,
        call_args_list: &[Vec<Type>],
        param_idx: usize,
    ) -> Option<Type> {
        // Only refine List<Unknown> for now
        if let Type::List(inner) = param_type {
            if matches!(inner.as_ref(), Type::Unknown) {
                // Find the most concrete type from all call sites
                for call_args in call_args_list {
                    if let Some(Type::List(elem_type)) = call_args.get(param_idx) {
                        if !matches!(elem_type.as_ref(), Type::Unknown) {
                            return Some(Type::List(elem_type.clone()));
                        }
                    }
                }
            }
        }
        None
    }

    /// Preprocess top-level statements to normalize main function and guard blocks
    fn preprocess_top_level(&self, stmts: &[Stmt]) -> Vec<Stmt> {
        let has_user_main = stmts.iter().any(|stmt| {
            matches!(stmt, Stmt::FuncDef { name, .. } if name == "main")
        });
        let mut new_stmts = Vec::new();
        // Pass: Flatten structure
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
                                if has_user_main {
                                    // User defines main: move it to _main_tsuchinoko and call it.
                                    new_stmts.push(Stmt::Expr(Expr::Call {
                                        func: Box::new(Expr::Ident("main".to_string())),
                                        args: vec![],
                                        kwargs: vec![],
                                    }));
                                } else {
                                    // No user main: inline guard body directly (no wrapper needed).
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

        new_stmts
    }

    pub fn analyze(&mut self, program: &Program) -> Result<Vec<IrNode>, TsuchinokoError> {
        // Step 1: Pre-processing (Declarative AST transformation)
        let stmts = self.preprocess_top_level(&program.statements);

        // Step 1.5: Collect mutable variables (targets of AugAssign or reassignment)
        self.collect_mutable_vars(&stmts);

        // Top-level hoisting setup
        self.hoisted_var_candidates.clear();
        self.func_base_depth = self.scope.depth();

        // Step 1.6: V1.5.2 - Pre-process imports to populate external_imports
        // This must be done BEFORE forward_declare_functions so that may_raise
        // can correctly detect external library calls.
        self.preprocess_imports(&stmts);

        // Step 1.7: V1.5.2 - Forward-declare all function signatures before analyzing statements
        // This ensures that function return types are available for type inference
        // when processing top-level code that calls functions defined later in the file.
        self.forward_declare_functions(&stmts);

        // Step 2: Unified Analysis (Pass 0 -> Pass 1)
        // Now top-level statements are treated exactly like block statements
        let ir_nodes = self.analyze_stmts(&stmts)?;
        let top_level_hoisted_vars: Vec<HoistedVar> = self
            .hoisted_var_candidates
            .drain()
            .map(|(name, (ty, _, _))| HoistedVar { name, ty })
            .collect();

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
                    hoisted_vars,
                    may_raise,
                    needs_bridge,
                } = other_decls.remove(pos)
                {
                    other_decls.push(IrNode::FuncDecl {
                        name: "__top_level__".to_string(),
                        params,
                        ret,
                        body,
                        hoisted_vars,
                        may_raise,
                        needs_bridge,
                    });
                }
            }
        } else {
            other_decls.push(IrNode::FuncDecl {
                name: "__top_level__".to_string(),
                params: vec![],
                ret: Type::Unit,
                body: main_body,
                hoisted_vars: top_level_hoisted_vars,
                may_raise: false,
                needs_bridge: self.current_func_needs_bridge,
            });
        }
        // V1.6.0 FT-005: If isinstance was used, generate DynamicValue enum at the top
        if !self.isinstance_types.is_empty() {
            let variants: Vec<(String, Type)> = self
                .isinstance_types
                .iter()
                .map(|ty| (self.type_to_dynamic_variant(ty), ty.clone()))
                .collect();

            let enum_def = IrNode::DynamicEnumDef {
                name: "DynamicValue".to_string(),
                variants,
            };

            // Insert enum definition at the beginning
            other_decls.insert(0, enum_def);
        }

        // Step 4: V1.7.0 Lowering Pass
        let module_aliases = self.module_global_aliases.clone();
        let lowering = crate::semantic::lowering::LoweringPass::new(
            module_aliases,
            self.type_table.clone(),
            self.expr_id_counter,
        );
        let lowered_nodes = lowering.apply(other_decls);

        Ok(lowered_nodes)
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
            // V1.5.0: Also check if value contains mutating method calls
            Stmt::Assign { target, value, .. } => {
                let exists_in_scope = self.scope.lookup(target).is_some();
                let seen_in_current_pass = seen_vars.contains(target);

                if exists_in_scope || seen_in_current_pass {
                    reassigned_vars.insert(target.clone());
                }
                seen_vars.insert(target.clone());

                // V1.5.0: Check for mutating method calls in the value expression
                // e.g., val = d.pop(2) - d needs to be mutable
                self.collect_expr_mutations(value, mutated_vars);
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
                                | "update" // V1.5.0: Dict methods
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
                else_body, // V1.5.2
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
                // V1.5.2: Check else_body
                if let Some(eb) = else_body {
                    for s in eb {
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

    /// V1.5.0: Recursively check expression for mutating method calls
    /// Used to detect mutations in assignment values, e.g., val = d.pop(2)
    fn collect_expr_mutations(
        &self,
        expr: &Expr,
        mutated_vars: &mut std::collections::HashSet<String>,
    ) {
        fn extract_base_var(expr: &Expr) -> Option<String> {
            match expr {
                Expr::Ident(name) => Some(name.clone()),
                Expr::Index { target, .. } => extract_base_var(target),
                _ => None,
            }
        }

        match expr {
            // d.pop(2) - check if method is mutating
            Expr::Call { func, args, .. } => {
                if let Expr::Attribute { value, attr } = func.as_ref() {
                    if let Some(name) = extract_base_var(value.as_ref()) {
                        if matches!(
                            attr.as_str(),
                            "append"
                                | "extend"
                                | "push"
                                | "pop"
                                | "insert"
                                | "remove"
                                | "clear"
                                | "add"
                                | "discard"
                                | "update"
                        ) {
                            mutated_vars.insert(name);
                        }
                    }
                    // Recurse into target
                    self.collect_expr_mutations(value, mutated_vars);
                }
                // Recurse into func and args
                self.collect_expr_mutations(func, mutated_vars);
                for arg in args {
                    self.collect_expr_mutations(arg, mutated_vars);
                }
            }
            // Recurse into sub-expressions
            Expr::BinOp { left, right, .. } => {
                self.collect_expr_mutations(left, mutated_vars);
                self.collect_expr_mutations(right, mutated_vars);
            }
            Expr::UnaryOp { operand, .. } => {
                self.collect_expr_mutations(operand, mutated_vars);
            }
            Expr::Tuple(elems) | Expr::List(elems) | Expr::Set(elems) => {
                for e in elems {
                    self.collect_expr_mutations(e, mutated_vars);
                }
            }
            Expr::Dict(pairs) => {
                for (k, v) in pairs {
                    self.collect_expr_mutations(k, mutated_vars);
                    self.collect_expr_mutations(v, mutated_vars);
                }
            }
            Expr::IfExp { test, body, orelse } => {
                self.collect_expr_mutations(test, mutated_vars);
                self.collect_expr_mutations(body, mutated_vars);
                self.collect_expr_mutations(orelse, mutated_vars);
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
                                    may_raise: false,
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

                let mut ty = match type_hint {
                    Some(th) => self.type_from_hint(th),
                    None => self.infer_type(value),
                };
                if type_hint.is_none() {
                    if let Expr::Call { func, .. } = value {
                        if let Expr::Ident(name) = func.as_ref() {
                            if name == "str" {
                                ty = Type::String;
                            }
                        }
                    }
                }

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
                    if self.scope.depth() > self.func_base_depth {
                        self.hoisted_var_candidates
                            .entry(target.clone())
                            .or_insert((ty.clone(), self.scope.depth(), self.func_base_depth));
                    }
                }

                let ir_value = self.analyze_expr(value)?;
                if type_hint.is_some() && !matches!(ty, Type::Any | Type::Unknown) {
                    self.set_type(ir_value.id, ty.clone());
                }

                // If type hint is concrete (String, Int, etc.) but expression is Type::Any,
                // wrap with JsonConversion for proper type conversion
                let expr_ty = self.infer_type(value);
                let is_bridge_value = match &ir_value.kind {
                    IrExprKind::BridgeCall { .. }
                    | IrExprKind::BridgeMethodCall { .. }
                    | IrExprKind::BridgeGet { .. }
                    | IrExprKind::BridgeAttributeAccess { .. }
                    | IrExprKind::BridgeItemAccess { .. }
                    | IrExprKind::BridgeSlice { .. } => true,
                    IrExprKind::Call { func, .. } => matches!(
                        func.kind,
                        IrExprKind::BridgeGet { .. }
                            | IrExprKind::BridgeAttributeAccess { .. }
                            | IrExprKind::BridgeItemAccess { .. }
                            | IrExprKind::BridgeSlice { .. }
                    ),
                    _ => false,
                };
                let mut skip_optional_wrap = false;
                let ir_value =
                    if matches!(expr_ty, Type::Any)
                        && !matches!(ty, Type::Any | Type::Unknown)
                        && !is_bridge_value
                    {
                        let conversion = match &ty {
                            Type::Float => Some("f64"),
                            Type::Int => Some("i64"),
                            Type::String => Some("String"),
                            Type::Bool => Some("bool"),
                            _ => None,
                        };
                        if let Some(conv) = conversion {
                            self.create_expr(IrExprKind::JsonConversion {
                                target: Box::new(ir_value),
                                convert_to: conv.to_string(),
                            }, ty.clone())
                        } else {
                            ir_value
                        }
                    } else {
                        ir_value
                    };

                // Bridge結果はLoweringでFromTnkValueを挿入するため、OptionalのSomeラップだけ抑止する。
                if matches!(ty, Type::Optional(_)) && is_bridge_value {
                    skip_optional_wrap = true;
                }

                // V1.5.0: If type hint is List with known element type, update IrExpr::List's elem_type
                // This ensures emitter can correctly add .to_string() for String elements in tuples
                // V1.7.0: Also wrap with TnkValue::from if target is Any/Optional<TnkValue> and value is not
                // This is properly implementing structured type conversion rather than ad-hoc fixes.
                
                let is_target_any_or_unknown = matches!(ty, Type::Any | Type::Unknown) 
                    || matches!(ty, Type::Optional(ref inner) if matches!(**inner, Type::Any | Type::Unknown));

                // If the value is explicitly a structured literal (Dict/List), we MUST wrap it
                // even if type inference says "Unknown" or "Any" (which might be imprecise).
                let is_value_structured_literal = matches!(ir_value.kind, IrExprKind::Dict {..} | IrExprKind::List {..} | IrExprKind::Tuple {..});

                let should_wrap = is_target_any_or_unknown
                    && (is_value_structured_literal || (!matches!(expr_ty, Type::Any) && !matches!(ir_value.kind, IrExprKind::TnkValueFrom(_) | IrExprKind::BridgeGet { .. })));
                
                let ir_value = if should_wrap {
                    self.create_expr(IrExprKind::TnkValueFrom(Box::new(ir_value)), Type::Any)
                } else {
                    ir_value
                };

                // V1.3.0: If type hint is List with known element type, update IrExpr::List's elem_type
                let ir_value = if let Type::List(elem_ty) = &ty {
                    if let IrExprKind::List {
                        elem_type: _,
                        elements,
                    } = ir_value.kind
                    {
                        self.create_expr(IrExprKind::List {
                            elem_type: *elem_ty.clone(),
                            elements,
                        }, ty.clone())
                    } else {
                        ir_value
                    }
                } else {
                    ir_value
                };

                // V1.5.0: Wrap non-None values in Some() when assigning to Optional type
                let ir_value = if matches!(ty, Type::Optional(_))
                    && !skip_optional_wrap
                    && !matches!(value, Expr::NoneLiteral)
                    && !matches!(expr_ty, Type::Optional(_))
                {
                    // If value is StringLit, add .to_string()
                    let ir_value = if matches!(ir_value.kind, IrExprKind::StringLit(_)) {
                        self.create_expr(IrExprKind::MethodCall {
                            target_type: Type::Unknown,
                            target: Box::new(ir_value),
                            method: "to_string".to_string(),
                            args: vec![],
                            callee_needs_bridge: false,
                        }, Type::String)
                    } else {
                        ir_value
                    };
                    let some_func = self.create_expr(IrExprKind::Var("Some".to_string()), Type::Unknown);
                    self.create_expr(IrExprKind::Call {
                        func: Box::new(some_func),
                        args: vec![ir_value],
                        callee_may_raise: false,
                        callee_needs_bridge: false,
                    }, ty.clone())
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
                    // V1.6.0 FT-008: PyO3 タプルアンパッキング
                    // Type::Any (PyO3 戻り値) の場合は、個別のインデックスアクセスに展開
                    if matches!(result_type, Type::Any) {
                        let mut nodes = Vec::new();

                        // まず一時変数に結果を格納
                        let temp_var = "_tuple_result".to_string();
                        nodes.push(IrNode::VarDecl {
                            name: temp_var.clone(),
                            ty: Type::Any,
                            mutable: false,
                            init: Some(Box::new(ir_value)),
                        });

                        // 各要素をインデックスアクセスで取得
                        for (i, target) in targets.iter().enumerate() {
                            let is_mutable = reassigned_vars.contains(target);
                            self.scope.define(target, Type::Any, is_mutable);
                            if self.scope.depth() > self.func_base_depth {
                                self.hoisted_var_candidates
                                    .entry(target.clone())
                                    .or_insert((Type::Any, self.scope.depth(), self.func_base_depth));
                            }

                            let temp_var_expr = self.create_expr(IrExprKind::Var(temp_var.clone()), Type::Any);
                            let i_expr = self.create_expr(IrExprKind::IntLit(i as i64), Type::Int);
                            let cast_expr = self.create_expr(IrExprKind::Cast {
                                target: Box::new(i_expr),
                                ty: "usize".to_string(),
                            }, Type::Unknown);
                            let index_expr = self.create_expr(IrExprKind::Index {
                                target: Box::new(temp_var_expr),
                                index: Box::new(cast_expr),
                            }, Type::Any);

                            let index_access = self.create_expr(IrExprKind::MethodCall {
                                target_type: Type::Any,
                                target: Box::new(index_expr),
                                method: "clone".to_string(),
                                args: vec![],
                                callee_needs_bridge: false,
                            }, Type::Any);
                            nodes.push(IrNode::VarDecl {
                                name: target.clone(),
                                ty: Type::Any,
                                mutable: is_mutable,
                                init: Some(Box::new(index_access)),
                            });
                        }

                        return Ok(IrNode::Sequence(nodes));
                    }

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
                        if self.scope.depth() > self.func_base_depth {
                            self.hoisted_var_candidates
                                .entry(target.clone())
                                .or_insert((ty.clone(), self.scope.depth(), self.func_base_depth));
                        }
                        decl_targets.push((target.clone(), ty, is_mutable));
                    }

                    // If value is a List, convert to tuple of indexed accesses
                    let final_value = if is_list {
                        let mut indices = Vec::new();
                        for i in 0..targets.len() {
                            let i_expr = self.create_expr(IrExprKind::IntLit(i as i64), Type::Int);
                            let cast_expr = self.create_expr(IrExprKind::Cast {
                                target: Box::new(i_expr),
                                ty: "usize".to_string(),
                            }, Type::Unknown);
                            let index_expr = self.create_expr(IrExprKind::Index {
                                target: Box::new(ir_value.clone()),
                                index: Box::new(cast_expr),
                            }, Type::Any);
                            indices.push(index_expr);
                        }
                        self.create_expr(IrExprKind::Tuple(indices), Type::Any)
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

    /// V1.6.0 FT-005: Extract isinstance check: isinstance(x, T) -> (x, T)
    fn extract_isinstance_check(&mut self, condition: &Expr) -> Option<(String, Type)> {
        if let Expr::Call { func, args, .. } = condition {
            if let Expr::Ident(name) = func.as_ref() {
                if name == "isinstance" && args.len() == 2 {
                    // isinstance(x, T)
                    if let Expr::Ident(var_name) = &args[0] {
                        let ty = match &args[1] {
                            Expr::Ident(type_name) => match type_name.as_str() {
                                "int" => Some(Type::Int),
                                "str" => Some(Type::String),
                                "float" => Some(Type::Float),
                                "bool" => Some(Type::Bool),
                                "list" => Some(Type::List(Box::new(Type::Unknown))),
                                "dict" => Some(Type::Dict(
                                    Box::new(Type::Unknown),
                                    Box::new(Type::Unknown),
                                )),
                                _ => None,
                            },
                            _ => None,
                        };
                        if let Some(t) = ty {
                            // V1.6.0: Track this type for DynamicValue enum generation
                            if !self.isinstance_types.contains(&t) {
                                self.isinstance_types.push(t.clone());
                            }
                            return Some((var_name.clone(), t));
                        }
                    }
                }
            }
        }
        None
    }

    /// V1.6.0 FT-005: Convert Type to DynamicValue variant name
    fn type_to_dynamic_variant(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "Int".to_string(),
            Type::String => "Str".to_string(),
            Type::Float => "Float".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::List(_) => "List".to_string(),
            Type::Dict(_, _) => "Dict".to_string(),
            _ => "Other".to_string(),
        }
    }
    fn get_func_name_for_debug(&self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(name) => name.clone(),
            Expr::Attribute { attr, .. } => attr.clone(),
            _ => "complex_call".to_string(),
        }
    }

}


#[cfg(test)]
mod tests;
