use crate::bridge::module_table::{get_import_mode, ImportMode};
use crate::ir::{IrExpr, IrExprKind, IrNode};
use crate::semantic::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct EmitPlan {
    pub needs_resident: bool,
    pub uses_tsuchinoko_error: bool,
    pub tnk_stub_needed: bool,
    pub func_plans: HashMap<String, FuncEmitPlan>,
    pub method_plans: HashMap<String, FuncEmitPlan>,
}

#[derive(Debug, Clone, Default)]
pub struct FuncEmitPlan {
    pub needs_bridge: bool,
    pub needs_resident: bool,
    pub returns_result: bool,
    pub uses_tsuchinoko_error: bool,
}

impl FuncEmitPlan {
    pub fn fallback(may_raise: bool, needs_bridge: bool, is_top_level: bool) -> Self {
        let needs_resident = needs_bridge;
        let returns_result = if is_top_level {
            true
        } else {
            may_raise || needs_resident
        };
        let uses_tsuchinoko_error = returns_result;
        Self {
            needs_bridge,
            needs_resident,
            returns_result,
            uses_tsuchinoko_error,
        }
    }
}

type AliasMap = HashMap<String, String>;

pub fn build_emit_plan(nodes: &[IrNode]) -> EmitPlan {
    let mut plan = EmitPlan::default();
    let mut tnk_usage = false;
    let mut type_uses_error = false;
    let alias_map = collect_aliases(nodes);

    for node in nodes {
        process_node_for_plan(
            node,
            &alias_map,
            &mut plan,
            &mut tnk_usage,
            &mut type_uses_error,
        );
    }

    plan.uses_tsuchinoko_error |= type_uses_error;
    plan.tnk_stub_needed = tnk_usage && !plan.needs_resident;
    plan
}

fn process_node_for_plan(
    node: &IrNode,
    alias_map: &AliasMap,
    plan: &mut EmitPlan,
    tnk_usage: &mut bool,
    type_uses_error: &mut bool,
) {
    match node {
        IrNode::FuncDecl {
            name,
            params,
            ret,
            body,
            may_raise,
            needs_bridge,
            ..
        } => {
            let (func_plan, func_tnk, func_type_error) = build_func_plan(
                name == "__top_level__",
                *may_raise,
                *needs_bridge,
                params,
                ret,
                body,
                alias_map,
            );
            plan.needs_resident |= func_plan.needs_resident;
            plan.uses_tsuchinoko_error |= func_plan.uses_tsuchinoko_error;
            plan.func_plans.insert(name.clone(), func_plan);
            *tnk_usage |= func_tnk;
            *type_uses_error |= func_type_error;
        }
        IrNode::ImplBlock {
            struct_name,
            methods,
        } => {
            for method in methods {
                if let IrNode::MethodDecl {
                    name,
                    params,
                    ret,
                    body,
                    may_raise,
                    needs_bridge,
                    ..
                } = method
                {
                    let (method_plan, method_tnk, method_type_error) = build_func_plan(
                        false,
                        *may_raise,
                        *needs_bridge,
                        params,
                        ret,
                        body,
                        alias_map,
                    );
                    plan.needs_resident |= method_plan.needs_resident;
                    plan.uses_tsuchinoko_error |= method_plan.uses_tsuchinoko_error;
                    plan.method_plans
                        .insert(format!("{}::{}", struct_name, name), method_plan);
                    *tnk_usage |= method_tnk;
                    *type_uses_error |= method_type_error;
                }
            }
        }
        IrNode::Sequence(nodes) => {
            for node in nodes {
                process_node_for_plan(node, alias_map, plan, tnk_usage, type_uses_error);
            }
        }
        _ => {
            let mut flags = ScanFlags::default();
            scan_node(node, &mut flags, alias_map);
            plan.needs_resident |= flags.uses_bridge;
            *tnk_usage |= flags.tnk_usage;
            *type_uses_error |= flags.type_uses_error;
        }
    }
}

fn build_func_plan(
    is_top_level: bool,
    may_raise: bool,
    needs_bridge: bool,
    params: &[(String, Type)],
    ret: &Type,
    body: &[IrNode],
    alias_map: &AliasMap,
) -> (FuncEmitPlan, bool, bool) {
    let mut flags = ScanFlags::default();
    for (_, ty) in params {
        scan_type(ty, &mut flags);
    }
    scan_type(ret, &mut flags);
    for node in body {
        scan_node(node, &mut flags, alias_map);
    }

    let needs_resident = flags.uses_bridge;
    let returns_result = if is_top_level {
        true
    } else {
        may_raise || needs_resident
    };
    let needs_bridge = needs_bridge || flags.uses_bridge;
    let uses_error_from_result = if is_top_level {
        may_raise || flags.uses_bridge
    } else {
        returns_result
    };
    let uses_tsuchinoko_error = uses_error_from_result || flags.type_uses_error;

    (
        FuncEmitPlan {
            needs_bridge,
            needs_resident,
            returns_result,
            uses_tsuchinoko_error,
        },
        flags.tnk_usage,
        flags.type_uses_error,
    )
}

#[derive(Debug, Default)]
struct ScanFlags {
    uses_bridge: bool,
    tnk_usage: bool,
    type_uses_error: bool,
}

fn scan_node(node: &IrNode, flags: &mut ScanFlags, alias_map: &AliasMap) {
    match node {
        IrNode::VarDecl { ty, init, .. } => {
            scan_type(ty, flags);
            if let Some(expr) = init {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrNode::Assign { value, .. } => scan_expr(value, flags, alias_map),
        IrNode::IndexAssign {
            target,
            index,
            value,
        } => {
            scan_expr(target, flags, alias_map);
            scan_expr(index, flags, alias_map);
            scan_expr(value, flags, alias_map);
        }
        IrNode::AugAssign { value, .. } => scan_expr(value, flags, alias_map),
        IrNode::MultiAssign { value, .. } => scan_expr(value, flags, alias_map),
        IrNode::MultiVarDecl { targets, value } => {
            for (_, ty, _) in targets {
                scan_type(ty, flags);
            }
            scan_expr(value, flags, alias_map);
        }
        IrNode::FieldAssign { target, value, .. } => {
            scan_expr(target, flags, alias_map);
            scan_expr(value, flags, alias_map);
        }
        IrNode::FuncDecl { .. } => {}
        IrNode::MethodDecl { .. } => {}
        IrNode::If {
            cond,
            then_block,
            else_block,
        } => {
            scan_expr(cond, flags, alias_map);
            for node in then_block {
                scan_node(node, flags, alias_map);
            }
            if let Some(block) = else_block {
                for node in block {
                    scan_node(node, flags, alias_map);
                }
            }
        }
        IrNode::For {
            var_type,
            iter,
            body,
            ..
        }
        | IrNode::BridgeBatchFor {
            var_type,
            iter,
            body,
            ..
        } => {
            scan_type(var_type, flags);
            scan_expr(iter, flags, alias_map);
            for node in body {
                scan_node(node, flags, alias_map);
            }
        }
        IrNode::While { cond, body } => {
            scan_expr(cond, flags, alias_map);
            for node in body {
                scan_node(node, flags, alias_map);
            }
        }
        IrNode::Return(expr) => {
            if let Some(expr) = expr {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrNode::StructDef { fields, .. } => {
            for (_, ty) in fields {
                scan_type(ty, flags);
            }
        }
        IrNode::ImplBlock { methods, .. } => {
            for method in methods {
                scan_node(method, flags, alias_map);
            }
        }
        IrNode::TypeAlias { ty, .. } => scan_type(ty, flags),
        IrNode::TryBlock {
            try_body,
            except_body,
            else_body,
            finally_body,
            ..
        } => {
            for node in try_body {
                scan_node(node, flags, alias_map);
            }
            for node in except_body {
                scan_node(node, flags, alias_map);
            }
            if let Some(nodes) = else_body {
                for node in nodes {
                    scan_node(node, flags, alias_map);
                }
            }
            if let Some(nodes) = finally_body {
                for node in nodes {
                    scan_node(node, flags, alias_map);
                }
            }
        }
        IrNode::Assert { test, msg } => {
            scan_expr(test, flags, alias_map);
            if let Some(expr) = msg {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrNode::Raise { message, cause, .. } => {
            flags.type_uses_error = true;
            scan_expr(message, flags, alias_map);
            if let Some(expr) = cause {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrNode::Expr(expr) => scan_expr(expr, flags, alias_map),
        IrNode::Sequence(nodes) => {
            for node in nodes {
                scan_node(node, flags, alias_map);
            }
        }
        IrNode::BridgeImport { .. } => {
            flags.uses_bridge = true;
        }
        IrNode::Block { stmts } => {
            for node in stmts {
                scan_node(node, flags, alias_map);
            }
        }
        IrNode::DynamicEnumDef { variants, .. } => {
            for (_, ty) in variants {
                scan_type(ty, flags);
            }
        }
        IrNode::Match { value, arms } => {
            scan_expr(value, flags, alias_map);
            for arm in arms {
                for node in &arm.body {
                    scan_node(node, flags, alias_map);
                }
            }
        }
        IrNode::Break | IrNode::Continue => {}
    }
}

fn scan_expr(expr: &IrExpr, flags: &mut ScanFlags, alias_map: &AliasMap) {
    match &expr.kind {
        IrExprKind::Call {
            func,
            args,
            callee_needs_bridge,
            ..
        } => {
            if *callee_needs_bridge {
                flags.uses_bridge = true;
            }
            scan_expr(func, flags, alias_map);
            for arg in args {
                scan_expr(arg, flags, alias_map);
            }
        }
        IrExprKind::MethodCall {
            target,
            args,
            callee_needs_bridge,
            ..
        } => {
            if *callee_needs_bridge {
                flags.uses_bridge = true;
            }
            scan_expr(target, flags, alias_map);
            for arg in args {
                scan_expr(arg, flags, alias_map);
            }
        }
        IrExprKind::PyO3Call { args, .. } => {
            if let IrExprKind::PyO3Call { module, method, .. } = &expr.kind {
                let real_module = alias_map
                    .get(module)
                    .cloned()
                    .unwrap_or_else(|| module.clone());
                let target = format!("{real_module}.{method}");
                let import_mode = get_import_mode(&target);
                if matches!(import_mode, ImportMode::PyO3 | ImportMode::Resident) {
                    flags.uses_bridge = true;
                }
            }
            for arg in args {
                scan_expr(arg, flags, alias_map);
            }
        }
        IrExprKind::PyO3MethodCall { target, args, .. } => {
            flags.uses_bridge = true;
            scan_expr(target, flags, alias_map);
            for arg in args {
                scan_expr(arg, flags, alias_map);
            }
        }
        IrExprKind::BridgeMethodCall {
            target,
            args,
            keywords,
            ..
        } => {
            flags.uses_bridge = true;
            scan_expr(target, flags, alias_map);
            for arg in args {
                scan_expr(arg, flags, alias_map);
            }
            for (_, value) in keywords {
                scan_expr(value, flags, alias_map);
            }
        }
        IrExprKind::BridgeCall {
            target,
            args,
            keywords,
        } => {
            flags.uses_bridge = true;
            scan_expr(target, flags, alias_map);
            for arg in args {
                scan_expr(arg, flags, alias_map);
            }
            for (_, value) in keywords {
                scan_expr(value, flags, alias_map);
            }
        }
        IrExprKind::BridgeAttributeAccess { target, .. } => {
            flags.uses_bridge = true;
            scan_expr(target, flags, alias_map);
        }
        IrExprKind::BridgeItemAccess { target, index } => {
            flags.uses_bridge = true;
            scan_expr(target, flags, alias_map);
            scan_expr(index, flags, alias_map);
        }
        IrExprKind::BridgeSlice {
            target,
            start,
            stop,
            step,
        } => {
            flags.uses_bridge = true;
            scan_expr(target, flags, alias_map);
            scan_expr(start, flags, alias_map);
            scan_expr(stop, flags, alias_map);
            scan_expr(step, flags, alias_map);
        }
        IrExprKind::BridgeGet { .. } => {
            flags.uses_bridge = true;
        }
        IrExprKind::FromTnkValue { value, to_type } => {
            flags.tnk_usage = true;
            scan_expr(value, flags, alias_map);
            scan_type(to_type, flags);
        }
        IrExprKind::TnkValueFrom(inner) => {
            flags.tnk_usage = true;
            scan_expr(inner, flags, alias_map);
        }
        IrExprKind::Ref(inner)
        | IrExprKind::Unwrap(inner)
        | IrExprKind::BoxNew(inner)
        | IrExprKind::Reference { target: inner }
        | IrExprKind::MutReference { target: inner } => {
            scan_expr(inner, flags, alias_map);
        }
        IrExprKind::BuiltinCall { args, .. } => {
            for arg in args {
                scan_expr(arg, flags, alias_map);
            }
        }
        IrExprKind::Sorted { iter, key, .. } => {
            scan_expr(iter, flags, alias_map);
            if let Some(key) = key {
                scan_expr(key, flags, alias_map);
            }
        }
        IrExprKind::ListComp {
            elt,
            iter,
            condition,
            ..
        }
        | IrExprKind::SetComp {
            elt,
            iter,
            condition,
            ..
        } => {
            scan_expr(elt, flags, alias_map);
            scan_expr(iter, flags, alias_map);
            if let Some(cond) = condition {
                scan_expr(cond, flags, alias_map);
            }
        }
        IrExprKind::DictComp {
            key,
            value,
            iter,
            condition,
            ..
        } => {
            scan_expr(key, flags, alias_map);
            scan_expr(value, flags, alias_map);
            scan_expr(iter, flags, alias_map);
            if let Some(cond) = condition {
                scan_expr(cond, flags, alias_map);
            }
        }
        IrExprKind::Index { target, index } => {
            scan_expr(target, flags, alias_map);
            scan_expr(index, flags, alias_map);
        }
        IrExprKind::Slice {
            target,
            start,
            end,
            step,
        } => {
            scan_expr(target, flags, alias_map);
            if let Some(expr) = start {
                scan_expr(expr, flags, alias_map);
            }
            if let Some(expr) = end {
                scan_expr(expr, flags, alias_map);
            }
            if let Some(expr) = step {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrExprKind::Range { start, end } => {
            scan_expr(start, flags, alias_map);
            scan_expr(end, flags, alias_map);
        }
        IrExprKind::Print { args } => {
            for (expr, ty) in args {
                scan_expr(expr, flags, alias_map);
                scan_type(ty, flags);
            }
        }
        IrExprKind::Closure { body, ret_type, .. } => {
            scan_type(ret_type, flags);
            for node in body {
                scan_node(node, flags, alias_map);
            }
        }
        IrExprKind::FString { values, .. } => {
            for (expr, ty) in values {
                scan_expr(expr, flags, alias_map);
                scan_type(ty, flags);
            }
        }
        IrExprKind::IfExp { test, body, orelse } => {
            scan_expr(test, flags, alias_map);
            scan_expr(body, flags, alias_map);
            scan_expr(orelse, flags, alias_map);
        }
        IrExprKind::Cast { target, .. } => {
            scan_expr(target, flags, alias_map);
        }
        IrExprKind::JsonConversion { target, .. } => {
            flags.tnk_usage = true;
            flags.type_uses_error = true;
            scan_expr(target, flags, alias_map);
        }
        IrExprKind::StructConstruct { fields, .. } => {
            for (_, expr) in fields {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrExprKind::DynamicWrap { value, .. } => {
            scan_expr(value, flags, alias_map);
        }
        IrExprKind::List {
            elem_type,
            elements,
        } => {
            scan_type(elem_type, flags);
            for expr in elements {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrExprKind::Tuple(elements) => {
            for expr in elements {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrExprKind::Dict {
            key_type,
            value_type,
            entries,
        } => {
            scan_type(key_type, flags);
            scan_type(value_type, flags);
            for (k, v) in entries {
                scan_expr(k, flags, alias_map);
                scan_expr(v, flags, alias_map);
            }
        }
        IrExprKind::Set {
            elem_type,
            elements,
        } => {
            scan_type(elem_type, flags);
            for expr in elements {
                scan_expr(expr, flags, alias_map);
            }
        }
        IrExprKind::FieldAccess { target, .. } => scan_expr(target, flags, alias_map),
        IrExprKind::BinOp { left, right, op } => {
            if matches!(op, crate::ir::IrBinOp::MatMul) {
                flags.uses_bridge = true;
            }
            scan_expr(left, flags, alias_map);
            scan_expr(right, flags, alias_map);
        }
        IrExprKind::UnaryOp { operand, .. } => scan_expr(operand, flags, alias_map),
        IrExprKind::StaticCall { args, .. } => {
            for arg in args {
                scan_expr(arg, flags, alias_map);
            }
        }
        IrExprKind::ConstRef { path: _ } => {}
        IrExprKind::RawCode(code) => {
            if code.contains("TnkValue") {
                flags.tnk_usage = true;
            }
            if code.contains("TsuchinokoError") {
                flags.type_uses_error = true;
            }
        }
        IrExprKind::Var(_)
        | IrExprKind::IntLit(_)
        | IrExprKind::FloatLit(_)
        | IrExprKind::StringLit(_)
        | IrExprKind::BoolLit(_)
        | IrExprKind::NoneLit => {}
    }
}

fn scan_type(ty: &Type, flags: &mut ScanFlags) {
    match ty {
        Type::Any | Type::Unknown => {
            flags.tnk_usage = true;
        }
        Type::List(inner)
        | Type::Set(inner)
        | Type::Optional(inner)
        | Type::Ref(inner)
        | Type::MutRef(inner) => scan_type(inner, flags),
        Type::Tuple(types) => {
            for ty in types {
                scan_type(ty, flags);
            }
        }
        Type::Dict(key, value) => {
            scan_type(key, flags);
            scan_type(value, flags);
        }
        Type::Func {
            params,
            ret,
            may_raise,
            ..
        } => {
            if *may_raise {
                flags.type_uses_error = true;
            }
            for ty in params {
                scan_type(ty, flags);
            }
            scan_type(ret, flags);
        }
        Type::Struct(_) | Type::Unit | Type::Int | Type::Float | Type::String | Type::Bool => {}
    }
}

fn collect_aliases(nodes: &[IrNode]) -> AliasMap {
    let mut aliases = AliasMap::new();
    for node in nodes {
        collect_aliases_from_node(node, &mut aliases);
    }
    aliases
}

fn collect_aliases_from_node(node: &IrNode, aliases: &mut AliasMap) {
    match node {
        IrNode::BridgeImport {
            module,
            alias,
            items,
        } => {
            if items.is_none() {
                let alias_name = alias.clone().unwrap_or_else(|| module.clone());
                aliases.insert(alias_name, module.clone());
            }
        }
        IrNode::FuncDecl { body, .. } | IrNode::MethodDecl { body, .. } => {
            for node in body {
                collect_aliases_from_node(node, aliases);
            }
        }
        IrNode::ImplBlock { methods, .. } => {
            for method in methods {
                collect_aliases_from_node(method, aliases);
            }
        }
        IrNode::If {
            then_block,
            else_block,
            ..
        } => {
            for node in then_block {
                collect_aliases_from_node(node, aliases);
            }
            if let Some(nodes) = else_block {
                for node in nodes {
                    collect_aliases_from_node(node, aliases);
                }
            }
        }
        IrNode::For { body, .. }
        | IrNode::BridgeBatchFor { body, .. }
        | IrNode::While { body, .. } => {
            for node in body {
                collect_aliases_from_node(node, aliases);
            }
        }
        IrNode::TryBlock {
            try_body,
            except_body,
            else_body,
            finally_body,
            ..
        } => {
            for node in try_body {
                collect_aliases_from_node(node, aliases);
            }
            for node in except_body {
                collect_aliases_from_node(node, aliases);
            }
            if let Some(nodes) = else_body {
                for node in nodes {
                    collect_aliases_from_node(node, aliases);
                }
            }
            if let Some(nodes) = finally_body {
                for node in nodes {
                    collect_aliases_from_node(node, aliases);
                }
            }
        }
        IrNode::Block { stmts } | IrNode::Sequence(stmts) => {
            for node in stmts {
                collect_aliases_from_node(node, aliases);
            }
        }
        IrNode::Match { arms, .. } => {
            for arm in arms {
                for node in &arm.body {
                    collect_aliases_from_node(node, aliases);
                }
            }
        }
        IrNode::StructDef { .. }
        | IrNode::TypeAlias { .. }
        | IrNode::VarDecl { .. }
        | IrNode::Assign { .. }
        | IrNode::IndexAssign { .. }
        | IrNode::AugAssign { .. }
        | IrNode::MultiAssign { .. }
        | IrNode::MultiVarDecl { .. }
        | IrNode::FieldAssign { .. }
        | IrNode::Return(_)
        | IrNode::Break
        | IrNode::Continue
        | IrNode::Assert { .. }
        | IrNode::Raise { .. }
        | IrNode::Expr(_)
        | IrNode::DynamicEnumDef { .. } => {}
    }
}
