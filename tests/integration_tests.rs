//! Integration tests for Tsuchinoko transpiler

use tsuchinoko::emitter::emit;
use tsuchinoko::ir::{IrBinOp, IrExpr, IrNode};
use tsuchinoko::semantic::Type;

/// Test: Simple variable assignment
/// Python: x: int = 10
/// Rust:   let x: i64 = 10;
#[test]
fn test_simple_assignment_to_rust() {
    // Create IR directly (until parser is connected)
    let ir = vec![IrNode::VarDecl {
        name: "x".to_string(),
        ty: Type::Int,
        mutable: false,
        init: Some(Box::new(IrExpr::IntLit(10))),
    }];

    let result = emit(&ir);
    assert_eq!(result.trim(), "let x: i64 = 10i64;");
}

/// Test: Function definition
/// Python:
/// def add(a: int, b: int) -> int:
///     return a + b
///
/// Rust:
/// fn add(a: i64, b: i64) -> i64 {
///     return (a + b);
/// }
#[test]
fn test_function_def_to_rust() {
    let ir = vec![IrNode::FuncDecl {
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
    }];

    let result = emit(&ir);
    assert!(result.contains("fn add(a: i64, b: i64) -> i64"));
    assert!(result.contains("return (a + b)"));
}

/// Test: If statement
/// Python:
/// if x > 0:
///     y = 1
/// else:
///     y = 0
#[test]
fn test_if_statement_to_rust() {
    let ir = vec![IrNode::If {
        cond: Box::new(IrExpr::BinOp {
            left: Box::new(IrExpr::Var("x".to_string())),
            op: IrBinOp::Gt,
            right: Box::new(IrExpr::IntLit(0)),
        }),
        then_block: vec![IrNode::Assign {
            target: "y".to_string(),
            value: Box::new(IrExpr::IntLit(1)),
        }],
        else_block: Some(vec![IrNode::Assign {
            target: "y".to_string(),
            value: Box::new(IrExpr::IntLit(0)),
        }]),
    }];

    let result = emit(&ir);
    // Parentheses around conditions are now stripped
    assert!(result.contains("if x > 0"));
    assert!(result.contains("y = 1"));
    assert!(result.contains("else"));
    assert!(result.contains("y = 0"));
}

/// Test: For loop with range
/// Python: for i in range(10):
/// Rust:   for i in 0..10 {
#[test]
fn test_for_loop_to_rust() {
    let ir = vec![IrNode::For {
        var: "i".to_string(),
        var_type: Type::Int,
        iter: Box::new(IrExpr::Range {
            start: Box::new(IrExpr::IntLit(0)),
            end: Box::new(IrExpr::IntLit(10)),
        }),
        body: vec![IrNode::Expr(IrExpr::Call {
            callee_may_raise: false,
            func: Box::new(IrExpr::Var("println".to_string())),
            args: vec![IrExpr::Var("i".to_string())],
        })],
    }];

    let result = emit(&ir);
    // Range currently emits with i64 suffixes
    assert!(result.contains("for i in 0i64..10i64"));
}

/// Test: List/Vec creation
/// Python: nums: list[int] = [1, 2, 3]
/// Rust:   let nums: Vec<i64> = vec![1, 2, 3];
#[test]
fn test_list_to_vec() {
    let ir = vec![IrNode::VarDecl {
        name: "nums".to_string(),
        ty: Type::List(Box::new(Type::Int)),
        mutable: false,
        init: Some(Box::new(IrExpr::List {
            elem_type: Type::Int,
            elements: vec![IrExpr::IntLit(1), IrExpr::IntLit(2), IrExpr::IntLit(3)],
        })),
    }];

    let result = emit(&ir);
    assert!(result.contains("let nums: Vec<i64> = vec![1i64, 2i64, 3i64]"));
}

/// Test: Type conversion - Python type hints to Rust types
#[test]
fn test_type_conversions() {
    assert_eq!(Type::Int.to_rust_string(), "i64");
    assert_eq!(Type::Float.to_rust_string(), "f64");
    assert_eq!(Type::String.to_rust_string(), "String");
    assert_eq!(Type::Bool.to_rust_string(), "bool");
    assert_eq!(Type::List(Box::new(Type::Int)).to_rust_string(), "Vec<i64>");
    assert_eq!(
        Type::Optional(Box::new(Type::String)).to_rust_string(),
        "Option<String>"
    );
}

/// Test: Mutable variable
/// Python: x: int = 10; x = 20
/// Rust:   let mut x: i64 = 10; x = 20;
#[test]
fn test_mutable_variable() {
    let ir = vec![
        IrNode::VarDecl {
            name: "x".to_string(),
            ty: Type::Int,
            mutable: true,
            init: Some(Box::new(IrExpr::IntLit(10))),
        },
        IrNode::Assign {
            target: "x".to_string(),
            value: Box::new(IrExpr::IntLit(20)),
        },
    ];

    let result = emit(&ir);
    assert!(result.contains("let mut x: i64 = 10"));
    assert!(result.contains("x = 20"));
}
