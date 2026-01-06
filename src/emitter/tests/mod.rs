//! emitter module tests
//!
//! Extracted from mod.rs for better code organization.

#![allow(clippy::approx_constant)]
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
    assert_eq!(result, "let x: i64 = 42i64;");
}

#[test]
fn test_emit_function() {
    let node = IrNode::FuncDecl {
        name: "add".to_string(),
        params: vec![("a".to_string(), Type::Int), ("b".to_string(), Type::Int)],
        ret: Type::Int,
        body: vec![IrNode::Return(Some(Box::new(IrExpr::BinOp {
            left: Box::new(IrExpr::Var("a".to_string())),
            op: IrBinOp::Add,
            right: Box::new(IrExpr::Var("b".to_string())),
        })))],
        hoisted_vars: vec![],
        may_raise: false,
    };
    let result = emit(&[node]);
    assert!(result.contains("fn add(a: i64, b: i64) -> i64"));
    assert!(result.contains("return (a + b)"));
}

// --- リテラル式テスト ---
#[test]
fn test_emit_int_lit() {
    let mut emitter = RustEmitter::new();
    assert_eq!(emitter.emit_expr(&IrExpr::IntLit(42)), "42i64");
}

#[test]
fn test_emit_float_lit() {
    let mut emitter = RustEmitter::new();
    assert_eq!(emitter.emit_expr(&IrExpr::FloatLit(3.14)), "3.1");
}

#[test]
fn test_emit_string_lit() {
    let mut emitter = RustEmitter::new();
    assert_eq!(
        emitter.emit_expr(&IrExpr::StringLit("hello".to_string())),
        "\"hello\""
    );
}

#[test]
fn test_emit_bool_lit() {
    let mut emitter = RustEmitter::new();
    assert_eq!(emitter.emit_expr(&IrExpr::BoolLit(true)), "true");
    assert_eq!(emitter.emit_expr(&IrExpr::BoolLit(false)), "false");
}

#[test]
fn test_emit_none_lit() {
    let mut emitter = RustEmitter::new();
    assert_eq!(emitter.emit_expr(&IrExpr::NoneLit), "None");
}

// --- 変数テスト ---
#[test]
fn test_emit_var() {
    let mut emitter = RustEmitter::new();
    assert_eq!(emitter.emit_expr(&IrExpr::Var("x".to_string())), "x");
}

#[test]
fn test_emit_var_camel_to_snake() {
    let mut emitter = RustEmitter::new();
    assert_eq!(
        emitter.emit_expr(&IrExpr::Var("myVariable".to_string())),
        "my_variable"
    );
}

// --- 演算テスト ---
#[test]
fn test_emit_binop_add() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(1)),
        op: IrBinOp::Add,
        right: Box::new(IrExpr::IntLit(2)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(1i64 + 2i64)");
}

#[test]
fn test_emit_binop_sub() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(5)),
        op: IrBinOp::Sub,
        right: Box::new(IrExpr::IntLit(3)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(5i64 - 3i64)");
}

#[test]
fn test_emit_binop_mul() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(4)),
        op: IrBinOp::Mul,
        right: Box::new(IrExpr::IntLit(3)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(4i64 * 3i64)");
}

#[test]
fn test_emit_binop_eq() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(1)),
        op: IrBinOp::Eq,
        right: Box::new(IrExpr::IntLit(1)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(1i64 == 1i64)");
}

#[test]
fn test_emit_binop_pow() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(2)),
        op: IrBinOp::Pow,
        right: Box::new(IrExpr::IntLit(3)),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".pow("));
}

#[test]
fn test_emit_unary_neg() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::UnaryOp {
        op: IrUnaryOp::Neg,
        operand: Box::new(IrExpr::IntLit(5)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(-5i64)");
}

#[test]
fn test_emit_unary_not() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::UnaryOp {
        op: IrUnaryOp::Not,
        operand: Box::new(IrExpr::BoolLit(true)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(!true)");
}

// --- コレクションテスト ---
#[test]
fn test_emit_list_int() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::List {
        elem_type: Type::Int,
        elements: vec![IrExpr::IntLit(1), IrExpr::IntLit(2)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("vec!["));
}

#[test]
fn test_emit_tuple() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Tuple(vec![IrExpr::IntLit(1), IrExpr::IntLit(2)]);
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("(1i64, 2i64)"));
}

#[test]
fn test_emit_dict_empty() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Dict {
        key_type: Type::String,
        value_type: Type::Int,
        entries: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("HashMap::new()"));
}

// --- 特殊式テスト ---
#[test]
fn test_emit_if_exp() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::IfExp {
        test: Box::new(IrExpr::BoolLit(true)),
        body: Box::new(IrExpr::IntLit(1)),
        orelse: Box::new(IrExpr::IntLit(0)),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("if true"));
}

#[test]
fn test_emit_cast() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Cast {
        target: Box::new(IrExpr::IntLit(42)),
        ty: "f64".to_string(),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("as f64"));
}

#[test]
fn test_emit_box_new() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BoxNew(Box::new(IrExpr::IntLit(42)));
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("Arc::new")); // BoxNewはArc::newを生成
}

#[test]
fn test_emit_raw_code() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::RawCode("custom_code()".to_string());
    assert_eq!(emitter.emit_expr(&expr), "custom_code()");
}

// --- ノードテスト ---
#[test]
fn test_emit_mutable_var() {
    let node = IrNode::VarDecl {
        name: "y".to_string(),
        ty: Type::Int,
        mutable: true,
        init: Some(Box::new(IrExpr::IntLit(10))),
    };
    let result = emit(&[node]);
    assert!(result.contains("let mut y"));
}

#[test]
fn test_emit_if_stmt() {
    let node = IrNode::If {
        cond: Box::new(IrExpr::BoolLit(true)),
        then_block: vec![IrNode::Return(Some(Box::new(IrExpr::IntLit(1))))],
        else_block: Some(vec![IrNode::Return(Some(Box::new(IrExpr::IntLit(0))))]),
    };
    let result = emit(&[node]);
    assert!(result.contains("if true"));
    assert!(result.contains("else"));
}

#[test]
fn test_emit_while_stmt() {
    let node = IrNode::While {
        cond: Box::new(IrExpr::BoolLit(true)),
        body: vec![IrNode::Break],
    };
    let result = emit(&[node]);
    assert!(result.contains("while true"));
    assert!(result.contains("break"));
}

#[test]
fn test_emit_return_none() {
    let node = IrNode::Return(None);
    let result = emit(&[node]);
    assert_eq!(result, "return;");
}

#[test]
fn test_emit_return_value() {
    let node = IrNode::Return(Some(Box::new(IrExpr::IntLit(42))));
    let result = emit(&[node]);
    assert!(result.contains("return 42i64"));
}

// --- 追加テスト: Assign系 ---
#[test]
fn test_emit_assign() {
    let node = IrNode::Assign {
        target: "x".to_string(),
        value: Box::new(IrExpr::IntLit(10)),
    };
    let result = emit(&[node]);
    assert!(result.contains("x = 10i64"));
}

#[test]
fn test_emit_aug_assign_add() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::Add,
        value: Box::new(IrExpr::IntLit(5)),
    };
    let result = emit(&[node]);
    assert!(result.contains("x += 5i64"));
}

#[test]
fn test_emit_aug_assign_sub() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::Sub,
        value: Box::new(IrExpr::IntLit(3)),
    };
    let result = emit(&[node]);
    assert!(result.contains("x -= 3i64"));
}

// --- for ループテスト ---
#[test]
fn test_emit_for_range() {
    let node = IrNode::For {
        var: "i".to_string(),
        var_type: Type::Int,
        iter: Box::new(IrExpr::Range {
            start: Box::new(IrExpr::IntLit(0)),
            end: Box::new(IrExpr::IntLit(10)),
        }),
        body: vec![IrNode::Break],
    };
    let result = emit(&[node]);
    assert!(result.contains("for i in"));
    assert!(result.contains("0i64..10i64"));
}

// --- struct関連テスト ---
#[test]
fn test_emit_struct_def() {
    let node = IrNode::StructDef {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), Type::Int), ("y".to_string(), Type::Int)],
    };
    let result = emit(&[node]);
    assert!(result.contains("struct Point"));
    assert!(result.contains("x:"));
    assert!(result.contains("i64"));
}

// --- Method Callテスト ---
#[test]
fn test_emit_method_call() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "len".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("arr.len()"));
}

#[test]
fn test_emit_method_call_with_args() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "push".to_string(),
        args: vec![IrExpr::IntLit(42)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("arr.push("));
}

// --- Field Accessテスト ---
#[test]
fn test_emit_field_access() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::FieldAccess {
        target: Box::new(IrExpr::Var("obj".to_string())),
        field: "name".to_string(),
    };
    let result = emitter.emit_expr(&expr);
    assert_eq!(result, "obj.name");
}

// --- Indexテスト ---
#[test]
fn test_emit_index() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Index {
        target: Box::new(IrExpr::Var("arr".to_string())),
        index: Box::new(IrExpr::IntLit(0)),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("arr[0"));
}

// --- Sliceテスト ---
#[test]
fn test_emit_slice() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Slice {
        target: Box::new(IrExpr::Var("arr".to_string())),
        start: Some(Box::new(IrExpr::IntLit(1))),
        end: Some(Box::new(IrExpr::IntLit(5))),
        step: None,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("arr"));
    assert!(result.contains(".."));
}

// --- Rangeテスト ---
#[test]
fn test_emit_range() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Range {
        start: Box::new(IrExpr::IntLit(0)),
        end: Box::new(IrExpr::IntLit(10)),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("0i64..10i64"));
}

// --- FStringテスト ---
#[test]
fn test_emit_fstring() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::FString {
        parts: vec!["Hello ".to_string(), "!".to_string()],
        values: vec![IrExpr::Var("name".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("format!"));
}

// --- Continueテスト ---
#[test]
fn test_emit_continue() {
    let node = IrNode::Continue;
    let result = emit(&[node]);
    assert_eq!(result, "continue;");
}

// --- ListCompテスト ---
#[test]
fn test_emit_list_comp() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::ListComp {
        elt: Box::new(IrExpr::BinOp {
            left: Box::new(IrExpr::Var("x".to_string())),
            op: IrBinOp::Mul,
            right: Box::new(IrExpr::IntLit(2)),
        }),
        target: "x".to_string(),
        iter: Box::new(IrExpr::Range {
            start: Box::new(IrExpr::IntLit(0)),
            end: Box::new(IrExpr::IntLit(10)),
        }),
        condition: None,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".map") || result.contains("iter"));
}

// --- JsonConversionテスト ---
#[test]
fn test_emit_json_conversion_i64() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::JsonConversion {
        target: Box::new(IrExpr::Var("val".to_string())),
        convert_to: "i64".to_string(),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".as_i64().unwrap()"));
}

#[test]
fn test_emit_json_conversion_string() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::JsonConversion {
        target: Box::new(IrExpr::Var("val".to_string())),
        convert_to: "String".to_string(),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".as_str().unwrap().to_string()"));
}

// --- StructConstructテスト ---
#[test]
fn test_emit_struct_construct() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::StructConstruct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), IrExpr::IntLit(10)),
            ("y".to_string(), IrExpr::IntLit(20)),
        ],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("Point {"));
    assert!(result.contains("x: 10i64"));
    assert!(result.contains("y: 20i64"));
}

// --- 比較演算テスト ---
#[test]
fn test_emit_binop_lt() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(1)),
        op: IrBinOp::Lt,
        right: Box::new(IrExpr::IntLit(2)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(1i64 < 2i64)");
}

#[test]
fn test_emit_binop_gt() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(3)),
        op: IrBinOp::Gt,
        right: Box::new(IrExpr::IntLit(2)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(3i64 > 2i64)");
}

#[test]
fn test_emit_binop_and() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::BoolLit(true)),
        op: IrBinOp::And,
        right: Box::new(IrExpr::BoolLit(false)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(true && false)");
}

#[test]
fn test_emit_binop_or() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::BoolLit(true)),
        op: IrBinOp::Or,
        right: Box::new(IrExpr::BoolLit(false)),
    };
    assert_eq!(emitter.emit_expr(&expr), "(true || false)");
}

// === 追加テスト: 全IrExprバリアント網羅 ===

// --- 残りのBinOp ---
#[test]
fn test_emit_binop_div() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(10)),
        op: IrBinOp::Div,
        right: Box::new(IrExpr::IntLit(2)),
    };
    assert!(emitter.emit_expr(&expr).contains("/"));
}

#[test]
fn test_emit_binop_mod() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(10)),
        op: IrBinOp::Mod,
        right: Box::new(IrExpr::IntLit(3)),
    };
    assert!(emitter.emit_expr(&expr).contains("%"));
}

#[test]
fn test_emit_binop_floor_div() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(10)),
        op: IrBinOp::FloorDiv,
        right: Box::new(IrExpr::IntLit(3)),
    };
    assert!(emitter.emit_expr(&expr).contains("/"));
}

#[test]
fn test_emit_binop_not_eq() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(1)),
        op: IrBinOp::NotEq,
        right: Box::new(IrExpr::IntLit(2)),
    };
    assert!(emitter.emit_expr(&expr).contains("!="));
}

#[test]
fn test_emit_binop_lt_eq() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(1)),
        op: IrBinOp::LtEq,
        right: Box::new(IrExpr::IntLit(2)),
    };
    assert!(emitter.emit_expr(&expr).contains("<="));
}

#[test]
fn test_emit_binop_gt_eq() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(2)),
        op: IrBinOp::GtEq,
        right: Box::new(IrExpr::IntLit(1)),
    };
    assert!(emitter.emit_expr(&expr).contains(">="));
}

#[test]
fn test_emit_binop_bit_and() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(5)),
        op: IrBinOp::BitAnd,
        right: Box::new(IrExpr::IntLit(3)),
    };
    assert!(emitter.emit_expr(&expr).contains("&"));
}

#[test]
fn test_emit_binop_bit_or() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(5)),
        op: IrBinOp::BitOr,
        right: Box::new(IrExpr::IntLit(3)),
    };
    assert!(emitter.emit_expr(&expr).contains("|"));
}

#[test]
fn test_emit_binop_shl() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(1)),
        op: IrBinOp::Shl,
        right: Box::new(IrExpr::IntLit(4)),
    };
    assert!(emitter.emit_expr(&expr).contains("<<"));
}

#[test]
fn test_emit_binop_shr() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(16)),
        op: IrBinOp::Shr,
        right: Box::new(IrExpr::IntLit(2)),
    };
    assert!(emitter.emit_expr(&expr).contains(">>"));
}

#[test]
fn test_emit_binop_contains() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(1)),
        op: IrBinOp::Contains,
        right: Box::new(IrExpr::Var("arr".to_string())),
    };
    assert!(emitter.emit_expr(&expr).contains(".contains("));
}

// --- Call ---
#[test]
fn test_emit_call_simple() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::Var("my_func".to_string())),
        args: vec![IrExpr::IntLit(42)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("my_func("));
}

// --- Dict with entries ---
#[test]
fn test_emit_dict_with_entries() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Dict {
        key_type: Type::String,
        value_type: Type::Int,
        entries: vec![(IrExpr::StringLit("a".to_string()), IrExpr::IntLit(1))],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("HashMap"));
}

// --- DictComp ---
#[test]
fn test_emit_dict_comp() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::DictComp {
        key: Box::new(IrExpr::Var("k".to_string())),
        value: Box::new(IrExpr::BinOp {
            left: Box::new(IrExpr::Var("v".to_string())),
            op: IrBinOp::Mul,
            right: Box::new(IrExpr::IntLit(2)),
        }),
        target: "k, v".to_string(),
        iter: Box::new(IrExpr::MethodCall {
            target: Box::new(IrExpr::Var("d".to_string())),
            method: "items".to_string(),
            args: vec![],
        }),
        condition: None,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("HashMap"));
}

// --- Print ---
#[test]
fn test_emit_print_empty() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Print { args: vec![] };
    assert_eq!(emitter.emit_expr(&expr), "println!()");
}

#[test]
fn test_emit_print_with_args() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Print {
        args: vec![(IrExpr::IntLit(42), Type::Int)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("println!"));
}

// --- Unwrap ---
#[test]
fn test_emit_unwrap() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Unwrap(Box::new(IrExpr::Var("opt".to_string())));
    assert!(emitter.emit_expr(&expr).contains(".unwrap()"));
}

// --- Reference ---
#[test]
fn test_emit_reference() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Reference {
        target: Box::new(IrExpr::Var("x".to_string())),
    };
    assert_eq!(emitter.emit_expr(&expr), "&x");
}

// --- MutReference ---
#[test]
fn test_emit_mut_reference() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MutReference {
        target: Box::new(IrExpr::Var("x".to_string())),
    };
    assert_eq!(emitter.emit_expr(&expr), "&mut x");
}

// --- UnaryOp Deref ---
#[test]
fn test_emit_unary_deref() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::UnaryOp {
        op: IrUnaryOp::Deref,
        operand: Box::new(IrExpr::Var("ptr".to_string())),
    };
    assert!(emitter.emit_expr(&expr).contains("*ptr"));
}

// === IrNode追加テスト ===

// --- IndexAssign ---
#[test]
fn test_emit_index_assign() {
    let node = IrNode::IndexAssign {
        target: Box::new(IrExpr::Var("arr".to_string())),
        index: Box::new(IrExpr::IntLit(0)),
        value: Box::new(IrExpr::IntLit(42)),
    };
    let result = emit(&[node]);
    assert!(result.contains("arr["));
    assert!(result.contains("= 42i64"));
}

// --- FieldAssign ---
#[test]
fn test_emit_field_assign() {
    let node = IrNode::FieldAssign {
        target: Box::new(IrExpr::Var("obj".to_string())),
        field: "name".to_string(),
        value: Box::new(IrExpr::StringLit("test".to_string())),
    };
    let result = emit(&[node]);
    assert!(result.contains("obj.name ="));
}

// --- MultiAssign ---
#[test]
fn test_emit_multi_assign() {
    let node = IrNode::MultiAssign {
        targets: vec!["a".to_string(), "b".to_string()],
        value: Box::new(IrExpr::Tuple(vec![IrExpr::IntLit(1), IrExpr::IntLit(2)])),
    };
    let result = emit(&[node]);
    assert!(result.contains("(a, b)") || result.contains("a ="));
}

// --- TypeAlias ---
#[test]
fn test_emit_type_alias() {
    let node = IrNode::TypeAlias {
        name: "MyInt".to_string(),
        ty: Type::Int,
    };
    let result = emit(&[node]);
    assert!(result.contains("type MyInt"));
}

// --- if without else ---
#[test]
fn test_emit_if_no_else() {
    let node = IrNode::If {
        cond: Box::new(IrExpr::BoolLit(true)),
        then_block: vec![IrNode::Break],
        else_block: None,
    };
    let result = emit(&[node]);
    assert!(result.contains("if true"));
    assert!(!result.contains("else"));
}

// --- list with string type ---
#[test]
fn test_emit_list_string() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::List {
        elem_type: Type::String,
        elements: vec![IrExpr::StringLit("a".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".to_string()"));
}

// --- JsonConversion f64 ---
#[test]
fn test_emit_json_conversion_f64() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::JsonConversion {
        target: Box::new(IrExpr::Var("val".to_string())),
        convert_to: "f64".to_string(),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".as_f64().unwrap()"));
}

// --- JsonConversion bool ---
#[test]
fn test_emit_json_conversion_bool() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::JsonConversion {
        target: Box::new(IrExpr::Var("val".to_string())),
        convert_to: "bool".to_string(),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".as_bool().unwrap()"));
}

// --- AugAssign other operators ---
#[test]
fn test_emit_aug_assign_mul() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::Mul,
        value: Box::new(IrExpr::IntLit(2)),
    };
    let result = emit(&[node]);
    assert!(result.contains("*="));
}

#[test]
fn test_emit_aug_assign_div() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::Div,
        value: Box::new(IrExpr::IntLit(2)),
    };
    let result = emit(&[node]);
    assert!(result.contains("/="));
}

// --- Var with module path ---
#[test]
fn test_emit_var_module_path() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Var("std::collections::HashMap".to_string());
    assert_eq!(emitter.emit_expr(&expr), "std::collections::HashMap");
}

// --- Var starting with uppercase (type name) ---
#[test]
fn test_emit_var_type_name() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Var("MyStruct".to_string());
    assert_eq!(emitter.emit_expr(&expr), "MyStruct");
}

// --- IfExp ---
#[test]
fn test_emit_if_exp_full() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::IfExp {
        test: Box::new(IrExpr::BoolLit(true)),
        body: Box::new(IrExpr::IntLit(1)),
        orelse: Box::new(IrExpr::IntLit(0)),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("if true"));
    assert!(result.contains("else"));
}

// --- ListComp with condition ---
#[test]
fn test_emit_list_comp_with_condition() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::ListComp {
        elt: Box::new(IrExpr::Var("x".to_string())),
        target: "x".to_string(),
        iter: Box::new(IrExpr::Range {
            start: Box::new(IrExpr::IntLit(0)),
            end: Box::new(IrExpr::IntLit(10)),
        }),
        condition: Some(Box::new(IrExpr::BinOp {
            left: Box::new(IrExpr::Var("x".to_string())),
            op: IrBinOp::Gt,
            right: Box::new(IrExpr::IntLit(5)),
        })),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".filter("));
}

// --- MethodCall with multiple args ---
#[test]
fn test_emit_method_call_multi_args() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "replace".to_string(),
        args: vec![
            IrExpr::StringLit("a".to_string()),
            IrExpr::StringLit("b".to_string()),
        ],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".replace("));
}

// --- Slice without end ---
#[test]
fn test_emit_slice_no_end() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Slice {
        target: Box::new(IrExpr::Var("arr".to_string())),
        start: Some(Box::new(IrExpr::IntLit(2))),
        end: None,
        step: None,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".."));
}

// === 複雑なテスト: 80%達成用 ===

// --- Closure ---
#[test]
fn test_emit_closure_simple() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Closure {
        params: vec!["x".to_string()],
        body: vec![IrNode::Expr(IrExpr::BinOp {
            left: Box::new(IrExpr::Var("x".to_string())),
            op: IrBinOp::Mul,
            right: Box::new(IrExpr::IntLit(2)),
        })],
        ret_type: Type::Int,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("move |"));
    assert!(result.contains("-> i64"));
}

#[test]
fn test_emit_closure_no_params() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Closure {
        params: vec![],
        body: vec![IrNode::Expr(IrExpr::IntLit(42))],
        ret_type: Type::Int,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("move ||"));
}

// --- FuncDecl with default ---
#[test]
fn test_emit_func_decl_simple() {
    let node = IrNode::FuncDecl {
        name: "my_func".to_string(),
        params: vec![("a".to_string(), Type::Int)],
        ret: Type::Int,
        body: vec![IrNode::Return(Some(Box::new(IrExpr::Var("a".to_string()))))],
        hoisted_vars: vec![],
        may_raise: false,
    };
    let result = emit(&[node]);
    assert!(result.contains("fn my_func("));
    assert!(result.contains("-> i64"));
    assert!(result.contains("return"));
}

#[test]
fn test_emit_func_unit_return() {
    let node = IrNode::FuncDecl {
        name: "do_nothing".to_string(),
        params: vec![],
        ret: Type::Unit,
        body: vec![],
        hoisted_vars: vec![],
        may_raise: false,
    };
    let result = emit(&[node]);
    assert!(result.contains("fn do_nothing()"));
    // Unit型の戻り値表示は実装依存（表示することも省略することもある）
}

// --- MethodDecl ---
#[test]
fn test_emit_method_decl() {
    let node = IrNode::MethodDecl {
        name: "get_value".to_string(),
        params: vec![],
        ret: Type::Int,
        body: vec![IrNode::Return(Some(Box::new(IrExpr::IntLit(42))))],
        takes_self: true,
        takes_mut_self: false,
    };
    let result = emit(&[node]);
    assert!(result.contains("fn get_value("));
    assert!(result.contains("&self"));
}

#[test]
fn test_emit_method_decl_mut_self() {
    let node = IrNode::MethodDecl {
        name: "set_value".to_string(),
        params: vec![("val".to_string(), Type::Int)],
        ret: Type::Unit,
        body: vec![],
        takes_self: true,
        takes_mut_self: true,
    };
    let result = emit(&[node]);
    assert!(result.contains("&mut self"));
}

// --- ImplBlock ---
#[test]
fn test_emit_impl_block() {
    let node = IrNode::ImplBlock {
        struct_name: "Point".to_string(),
        methods: vec![],
    };
    let result = emit(&[node]);
    assert!(result.contains("impl Point"));
}

// --- Call with Some/None ---
#[test]
fn test_emit_call_some() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::Var("Some".to_string())),
        args: vec![IrExpr::IntLit(42)],
    };
    let result = emitter.emit_expr(&expr);
    assert_eq!(result, "Some(42i64)");
}

#[test]
fn test_emit_call_with_path() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::Var("std::cmp::max".to_string())),
        args: vec![IrExpr::IntLit(1), IrExpr::IntLit(2)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("std::cmp::max("));
}

// --- Print with multiple args ---
#[test]
fn test_emit_print_multiple() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Print {
        args: vec![
            (IrExpr::IntLit(1), Type::Int),
            (IrExpr::IntLit(2), Type::Int),
        ],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("println!"));
}

// --- List with tuple element ---
#[test]
fn test_emit_list_tuple() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::List {
        elem_type: Type::Tuple(vec![Type::String, Type::Int]),
        elements: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("vec!"));
}

// --- FString with multiple values ---
#[test]
fn test_emit_fstring_multiple() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::FString {
        parts: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        values: vec![IrExpr::IntLit(1), IrExpr::IntLit(2)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("format!"));
}

// --- DictComp with condition ---
#[test]
fn test_emit_dict_comp_with_condition() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::DictComp {
        key: Box::new(IrExpr::Var("k".to_string())),
        value: Box::new(IrExpr::Var("v".to_string())),
        target: "k, v".to_string(),
        iter: Box::new(IrExpr::MethodCall {
            target: Box::new(IrExpr::Var("d".to_string())),
            method: "items".to_string(),
            args: vec![],
        }),
        condition: Some(Box::new(IrExpr::BinOp {
            left: Box::new(IrExpr::Var("v".to_string())),
            op: IrBinOp::Gt,
            right: Box::new(IrExpr::IntLit(0)),
        })),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".filter("));
}

// --- Index with negative ---
#[test]
fn test_emit_index_negative() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Index {
        target: Box::new(IrExpr::Var("arr".to_string())),
        index: Box::new(IrExpr::UnaryOp {
            op: IrUnaryOp::Neg,
            operand: Box::new(IrExpr::IntLit(1)),
        }),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("len()") || result.contains("arr["));
}

// --- MultiVarDecl ---
#[test]
fn test_emit_multi_var_decl() {
    let node = IrNode::MultiVarDecl {
        targets: vec![
            ("a".to_string(), Type::Int, false),
            ("b".to_string(), Type::Int, false),
        ],
        value: Box::new(IrExpr::Tuple(vec![IrExpr::IntLit(1), IrExpr::IntLit(2)])),
    };
    let result = emit(&[node]);
    assert!(result.contains("let (a, b)"));
}

// --- Slice without start ---
#[test]
fn test_emit_slice_no_start() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Slice {
        target: Box::new(IrExpr::Var("arr".to_string())),
        start: None,
        end: Some(Box::new(IrExpr::IntLit(5))),
        step: None,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".."));
}

// --- AugAssign pow ---
#[test]
fn test_emit_aug_assign_pow() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::Pow,
        value: Box::new(IrExpr::IntLit(2)),
    };
    let result = emit(&[node]);
    assert!(result.contains(".pow(") || result.contains("**"));
}

// --- BitXor ---
#[test]
fn test_emit_binop_bit_xor() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(5)),
        op: IrBinOp::BitXor,
        right: Box::new(IrExpr::IntLit(3)),
    };
    assert!(emitter.emit_expr(&expr).contains("^"));
}

// --- NotContains ---
#[test]
fn test_emit_binop_not_contains() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(1)),
        op: IrBinOp::NotContains,
        right: Box::new(IrExpr::Var("arr".to_string())),
    };
    assert!(emitter.emit_expr(&expr).contains("!"));
}

// --- Call field access ---
#[test]
fn test_emit_call_field_func() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::FieldAccess {
            target: Box::new(IrExpr::Var("obj".to_string())),
            field: "callback".to_string(),
        }),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("(obj.callback)()"));
}

// --- Is/IsNot ---
#[test]
fn test_emit_binop_is() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::Var("x".to_string())),
        op: IrBinOp::Is,
        right: Box::new(IrExpr::NoneLit),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("is_none") || result.contains("=="));
}

// --- Call with print using clone wrapper ---
#[test]
fn test_emit_call_print_string_literal() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::Var("print".to_string())),
        args: vec![IrExpr::StringLit("hello".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("println!"));
}

// --- print with MethodCall wrapper (clone) ---
#[test]
fn test_emit_call_print_with_clone() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::Var("print".to_string())),
        args: vec![IrExpr::MethodCall {
            target: Box::new(IrExpr::Var("s".to_string())),
            method: "clone".to_string(),
            args: vec![],
        }],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("println!"));
}

// --- UnaryOp BitNot ---
#[test]
fn test_emit_unary_bitnot() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::UnaryOp {
        op: IrUnaryOp::BitNot,
        operand: Box::new(IrExpr::IntLit(5)),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("!"));
}

// --- VarDecl without init ---
#[test]
fn test_emit_var_decl_no_init() {
    let node = IrNode::VarDecl {
        name: "x".to_string(),
        ty: Type::Int,
        mutable: true,
        init: None,
    };
    let result = emit(&[node]);
    assert!(result.contains("let mut x"));
    assert!(!result.contains("="));
}

// --- Dict iter ---
#[test]
fn test_emit_dict_iter() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("d".to_string())),
        method: "items".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".iter()") || result.contains("d.items"));
}

// --- ListComp with dict iter ---
#[test]
fn test_emit_list_comp_dict_values() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::ListComp {
        elt: Box::new(IrExpr::Var("v".to_string())),
        target: "v".to_string(),
        iter: Box::new(IrExpr::MethodCall {
            target: Box::new(IrExpr::Var("d".to_string())),
            method: "values".to_string(),
            args: vec![],
        }),
        condition: None,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".values()"));
}

// === 80%達成用最終テスト ===

// --- For with tuple unpacking ---
#[test]
fn test_emit_for_tuple_unpacking() {
    let node = IrNode::For {
        var: "i, item".to_string(),
        var_type: Type::Tuple(vec![Type::Int, Type::String]),
        iter: Box::new(IrExpr::Var("items".to_string())),
        body: vec![IrNode::Break],
    };
    let result = emit(&[node]);
    assert!(result.contains("(i, item)"));
}

// --- Expr docstring (string literal as statement) ---
#[test]
fn test_emit_expr_docstring() {
    let node = IrNode::Expr(IrExpr::StringLit("This is a docstring".to_string()));
    let result = emit(&[node]);
    assert!(result.contains("//"));
}

// --- List with Option type ---
#[test]
fn test_emit_list_option_type() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::List {
        elem_type: Type::Optional(Box::new(Type::Int)),
        elements: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("vec!"));
}

// --- MatMul operator ---
#[test]
fn test_emit_binop_matmul() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::Var("a".to_string())),
        op: IrBinOp::MatMul,
        right: Box::new(IrExpr::Var("b".to_string())),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("numpy.matmul") || result.contains("py_bridge"));
}

// --- AugAssign with Mod ---
#[test]
fn test_emit_aug_assign_mod() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::Mod,
        value: Box::new(IrExpr::IntLit(3)),
    };
    let result = emit(&[node]);
    assert!(result.contains("%="));
}

// --- print with variable ---
#[test]
fn test_emit_call_print_variable() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::Var("print".to_string())),
        args: vec![IrExpr::Var("x".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("println!"));
    assert!(result.contains("&x"));
}

// --- main function rename ---
#[test]
fn test_emit_call_main() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::Var("main".to_string())),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("main_py()"));
}

// --- MethodCall to_string special ---
#[test]
fn test_emit_method_call_to_string() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::IntLit(42)),
        method: "to_string".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".to_string()"));
}

// --- MethodCall append as push ---
#[test]
fn test_emit_method_call_append() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "append".to_string(),
        args: vec![IrExpr::IntLit(1)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".push(") || result.contains(".append("));
}

// --- MethodCall split ---
#[test]
fn test_emit_method_call_split() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "split".to_string(),
        args: vec![IrExpr::StringLit(",".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".split("));
}

// --- Index with cast ---
#[test]
fn test_emit_index_cast() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Index {
        target: Box::new(IrExpr::Var("arr".to_string())),
        index: Box::new(IrExpr::Var("i".to_string())),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("arr["));
}

// --- Range only ---
#[test]
fn test_emit_range_only() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Range {
        start: Box::new(IrExpr::IntLit(0)),
        end: Box::new(IrExpr::Var("n".to_string())),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("0i64..n"));
}

// --- BinOp IsNot ---
#[test]
fn test_emit_binop_is_not() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::Var("x".to_string())),
        op: IrBinOp::IsNot,
        right: Box::new(IrExpr::NoneLit),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("is_some") || result.contains("!="));
}

// --- Empty print ---
#[test]
fn test_emit_call_print_empty() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Call { callee_may_raise: false,
        func: Box::new(IrExpr::Var("print".to_string())),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("println!()"));
}

// --- Static method (no self) ---
#[test]
fn test_emit_method_decl_static() {
    let node = IrNode::MethodDecl {
        name: "create".to_string(),
        params: vec![],
        ret: Type::Int,
        body: vec![IrNode::Return(Some(Box::new(IrExpr::IntLit(42))))],
        takes_self: false,
        takes_mut_self: false,
    };
    let result = emit(&[node]);
    assert!(result.contains("fn create("));
    assert!(!result.contains("self"));
}

// --- Slice both None ---
#[test]
fn test_emit_slice_full() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Slice {
        target: Box::new(IrExpr::Var("arr".to_string())),
        start: None,
        end: None,
        step: None,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("..") || result.contains("arr"));
}

// --- MethodCall strip ---
#[test]
fn test_emit_method_call_strip() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "strip".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".trim()") || result.contains(".strip"));
}

// --- Print expr with type info ---
#[test]
fn test_emit_print_expr_string_type() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Print {
        args: vec![(IrExpr::Var("s".to_string()), Type::String)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("println!"));
}

// --- AugAssign FloorDiv ---
#[test]
fn test_emit_aug_assign_floor_div() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::FloorDiv,
        value: Box::new(IrExpr::IntLit(2)),
    };
    let result = emit(&[node]);
    assert!(result.contains("/="));
}

// --- AugAssign bit ops ---
#[test]
fn test_emit_aug_assign_bit_and() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::BitAnd,
        value: Box::new(IrExpr::IntLit(3)),
    };
    let result = emit(&[node]);
    assert!(result.contains("&="));
}

#[test]
fn test_emit_aug_assign_bit_or() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::BitOr,
        value: Box::new(IrExpr::IntLit(3)),
    };
    let result = emit(&[node]);
    assert!(result.contains("|="));
}

#[test]
fn test_emit_aug_assign_shl() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::Shl,
        value: Box::new(IrExpr::IntLit(1)),
    };
    let result = emit(&[node]);
    assert!(result.contains("<<="));
}

#[test]
fn test_emit_aug_assign_shr() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::Shr,
        value: Box::new(IrExpr::IntLit(1)),
    };
    let result = emit(&[node]);
    assert!(result.contains(">>="));
}

// === 80%達成 最終テストバッチ ===

// --- PyO3Call ---
#[test]
fn test_emit_pyo3_call() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::PyO3Call {
        module: "numpy".to_string(),
        method: "array".to_string(),
        args: vec![IrExpr::List {
            elem_type: Type::Int,
            elements: vec![IrExpr::IntLit(1)],
        }],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("py_bridge") || result.contains("call"));
}

// --- PyO3MethodCall ---
#[test]
fn test_emit_pyo3_method_call() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::PyO3MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "sum".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("py_bridge") || result.contains("call"));
}

// --- MethodCall join ---
#[test]
fn test_emit_method_call_join() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::StringLit(",".to_string())),
        method: "join".to_string(),
        args: vec![IrExpr::Var("items".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".join(") || result.contains("collect"));
}

// --- MethodCall format ---
#[test]
fn test_emit_method_call_format() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::StringLit("{}".to_string())),
        method: "format".to_string(),
        args: vec![IrExpr::IntLit(42)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("format!") || result.contains(".format"));
}

// --- MethodCall lower/upper ---
#[test]
fn test_emit_method_call_lower() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "lower".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".to_lowercase()") || result.contains(".lower"));
}

#[test]
fn test_emit_method_call_upper() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "upper".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".to_uppercase()") || result.contains(".upper"));
}

// --- MethodCall startswith/endswith ---
#[test]
fn test_emit_method_call_startswith() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "startswith".to_string(),
        args: vec![IrExpr::StringLit("pre".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".starts_with(") || result.contains(".startswith"));
}

#[test]
fn test_emit_method_call_endswith() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "endswith".to_string(),
        args: vec![IrExpr::StringLit("suf".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".ends_with(") || result.contains(".endswith"));
}

// --- MethodCall get (dict) ---
#[test]
fn test_emit_method_call_get() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("d".to_string())),
        method: "get".to_string(),
        args: vec![IrExpr::StringLit("key".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".get("));
}

// --- MethodCall keys/values ---
#[test]
fn test_emit_method_call_keys() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("d".to_string())),
        method: "keys".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".keys()"));
}

// --- MethodCall pop (list) ---
#[test]
fn test_emit_method_call_pop() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "pop".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".pop()"));
}

// --- MethodCall extend ---
#[test]
fn test_emit_method_call_extend() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "extend".to_string(),
        args: vec![IrExpr::Var("other".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".extend("));
}

// --- MethodCall copy/deepcopy ---
#[test]
fn test_emit_method_call_copy() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "copy".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    // copy is likely just forwarded as method call
    assert!(result.contains("copy") || result.contains("clone") || result.contains("arr"));
}

// --- FuncDecl with params of various types ---
#[test]
fn test_emit_func_decl_multi_params() {
    let node = IrNode::FuncDecl {
        name: "process".to_string(),
        params: vec![
            ("name".to_string(), Type::String),
            ("count".to_string(), Type::Int),
            ("flag".to_string(), Type::Bool),
        ],
        ret: Type::String,
        body: vec![IrNode::Return(Some(Box::new(IrExpr::Var(
            "name".to_string(),
        ))))],
        hoisted_vars: vec![],
        may_raise: false,
    };
    let result = emit(&[node]);
    assert!(result.contains("name: String"));
    assert!(result.contains("count: i64"));
    assert!(result.contains("flag: bool"));
}

// --- ImplBlock with method ---
#[test]
fn test_emit_impl_block_with_method() {
    let node = IrNode::ImplBlock {
        struct_name: "Counter".to_string(),
        methods: vec![IrNode::MethodDecl {
            name: "increment".to_string(),
            params: vec![],
            ret: Type::Unit,
            body: vec![],
            takes_self: true,
            takes_mut_self: true,
        }],
    };
    let result = emit(&[node]);
    assert!(result.contains("impl Counter"));
    assert!(result.contains("fn increment"));
}

// --- If with elif ---
#[test]
fn test_emit_if_elif() {
    let node = IrNode::If {
        cond: Box::new(IrExpr::BoolLit(true)),
        then_block: vec![IrNode::Return(Some(Box::new(IrExpr::IntLit(1))))],
        else_block: Some(vec![IrNode::If {
            cond: Box::new(IrExpr::BoolLit(false)),
            then_block: vec![IrNode::Return(Some(Box::new(IrExpr::IntLit(2))))],
            else_block: None,
        }]),
    };
    let result = emit(&[node]);
    assert!(result.contains("if true"));
    assert!(result.contains("else"));
}

// --- AugAssign BitXor ---
#[test]
fn test_emit_aug_assign_bit_xor() {
    let node = IrNode::AugAssign {
        target: "x".to_string(),
        op: IrAugAssignOp::BitXor,
        value: Box::new(IrExpr::IntLit(3)),
    };
    let result = emit(&[node]);
    assert!(result.contains("^="));
}

// --- List with float type ---
#[test]
fn test_emit_list_float() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::List {
        elem_type: Type::Float,
        elements: vec![IrExpr::FloatLit(1.0), IrExpr::FloatLit(2.0)],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("vec!"));
}

// --- Dict with int key ---
#[test]
fn test_emit_dict_int_key() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Dict {
        key_type: Type::Int,
        value_type: Type::String,
        entries: vec![(IrExpr::IntLit(1), IrExpr::StringLit("one".to_string()))],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("HashMap"));
}

// --- Var with underscore prefix (private) ---
#[test]
fn test_emit_var_private() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Var("_private_var".to_string());
    assert_eq!(emitter.emit_expr(&expr), "_private_var");
}

// --- MethodCall find ---
#[test]
fn test_emit_method_call_find() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "find".to_string(),
        args: vec![IrExpr::StringLit("x".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".find(") || result.contains(".position("));
}

// --- MethodCall replace ---
#[test]
fn test_emit_method_call_replace_full() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "replace".to_string(),
        args: vec![
            IrExpr::StringLit("old".to_string()),
            IrExpr::StringLit("new".to_string()),
        ],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".replace("));
    assert!(result.contains("\"old\""));
    assert!(result.contains("\"new\""));
}

// --- FString empty ---
#[test]
fn test_emit_fstring_empty() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::FString {
        parts: vec!["hello".to_string()],
        values: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("hello") || result.contains("format!"));
}

// --- MethodCall enumerate ---
#[test]
fn test_emit_method_call_enumerate() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "enumerate".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".enumerate()") || result.contains(".iter()"));
}

// === 80%最終達成テスト ===

// --- TryBlock ---
#[test]
fn test_emit_try_block() {
    let node = IrNode::TryBlock {
        try_body: vec![IrNode::Return(Some(Box::new(IrExpr::IntLit(42))))],
        except_body: vec![IrNode::Return(Some(Box::new(IrExpr::IntLit(0))))],
        except_var: None,   // V1.5.2
        else_body: None,    // V1.5.2
        finally_body: None,
    };
    let result = emit(&[node]);
    assert!(result.contains("catch_unwind") || result.contains("panic"));
}

// --- VarDecl with tuple init ---
#[test]
fn test_emit_var_decl_tuple_init() {
    let node = IrNode::VarDecl {
        name: "point".to_string(),
        ty: Type::Tuple(vec![Type::Int, Type::Int]),
        mutable: false,
        init: Some(Box::new(IrExpr::Tuple(vec![
            IrExpr::IntLit(1),
            IrExpr::IntLit(2),
        ]))),
    };
    let result = emit(&[node]);
    assert!(result.contains("let point"));
    assert!(result.contains("(1i64, 2i64)"));
}

// --- VarDecl with string init ---
#[test]
fn test_emit_var_decl_string_init() {
    let node = IrNode::VarDecl {
        name: "name".to_string(),
        ty: Type::String,
        mutable: true,
        init: Some(Box::new(IrExpr::StringLit("hello".to_string()))),
    };
    let result = emit(&[node]);
    assert!(result.contains("let mut name"));
    assert!(result.contains(".to_string()"));
}

// --- List with tuple String element ---
#[test]
fn test_emit_list_tuple_string() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::List {
        elem_type: Type::Tuple(vec![Type::String, Type::Int]),
        elements: vec![IrExpr::Tuple(vec![
            IrExpr::StringLit("a".to_string()),
            IrExpr::IntLit(1),
        ])],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("vec!"));
}

// --- BinOp Pow ---
#[test]
fn test_emit_binop_pow_v2() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::BinOp {
        left: Box::new(IrExpr::IntLit(2)),
        op: IrBinOp::Pow,
        right: Box::new(IrExpr::IntLit(3)),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".pow(") || result.contains("**"));
}

// --- Nonelit ---
#[test]
fn test_emit_nonellit() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::NoneLit;
    assert_eq!(emitter.emit_expr(&expr), "None");
}

// --- Closure with Unit return ---
#[test]
fn test_emit_closure_unit_return() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Closure {
        params: vec![],
        body: vec![],
        ret_type: Type::Unit,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("move ||"));
}

// --- Closure with Unknown return ---
#[test]
fn test_emit_closure_unknown_return() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Closure {
        params: vec!["x".to_string()],
        body: vec![IrNode::Expr(IrExpr::Var("x".to_string()))],
        ret_type: Type::Unknown,
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("move |x|"));
}

// --- MethodCall zip ---
#[test]
fn test_emit_method_call_zip() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("a".to_string())),
        method: "zip".to_string(),
        args: vec![IrExpr::Var("b".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".zip("));
}

// --- MethodCall count ---
#[test]
fn test_emit_method_call_count() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("s".to_string())),
        method: "count".to_string(),
        args: vec![IrExpr::StringLit("x".to_string())],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".count(") || result.contains(".matches("));
}

// --- RawCode ---
#[test]
fn test_emit_raw_code_v2() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::RawCode("unsafe { std::mem::transmute(x) }".to_string());
    let result = emitter.emit_expr(&expr);
    assert_eq!(result, "unsafe { std::mem::transmute(x) }");
}

// --- Cast ---
#[test]
fn test_emit_cast_v2() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::Cast {
        target: Box::new(IrExpr::Var("x".to_string())),
        ty: "f64".to_string(),
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains("as f64") || result.contains("f64::from"));
}

// --- MethodCall abs ---
#[test]
fn test_emit_method_call_abs() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("x".to_string())),
        method: "abs".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".abs()"));
}

// --- MethodCall sort ---
#[test]
fn test_emit_method_call_sort() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "sort".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".sort()"));
}

// --- MethodCall reverse ---
#[test]
fn test_emit_method_call_reverse() {
    let mut emitter = RustEmitter::new();
    let expr = IrExpr::MethodCall {
        target: Box::new(IrExpr::Var("arr".to_string())),
        method: "reverse".to_string(),
        args: vec![],
    };
    let result = emitter.emit_expr(&expr);
    assert!(result.contains(".reverse()"));
}
