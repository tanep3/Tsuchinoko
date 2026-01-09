use crate::ir::{IrExpr, IrNode};
use crate::parser::parse;
use crate::semantic::analyze;

#[test]
fn test_bridge_method_call() {
    let code = r#"
def test(x: Any):
    x.foo(1, "str")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();

    // Check IR structure
    if let IrNode::FuncDecl { body, .. } = &ir[0] {
        // x.foo returns Any -> IrExpr inside
        // body[0] is Expr stmt
        if let IrNode::Expr(expr) = &body[0] {
            if let IrExpr::BridgeMethodCall { target: _, method, args } = expr {
                assert_eq!(method, "foo");
                assert_eq!(args.len(), 2);
                return;
            }
        }
    }
    panic!("Expected BridgeMethodCall");
}

#[test]
fn test_bridge_attribute_access() {
    let code = r#"
def test(x: Any):
    val = x.attr
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();

    if let IrNode::FuncDecl { body, .. } = &ir[0] {
        if let IrNode::VarDecl { init: Some(expr), .. } = &body[0] {
            if let IrExpr::BridgeAttributeAccess { target: _, attribute } = expr.as_ref() {
                assert_eq!(attribute, "attr");
                return;
            }
        }
    }
    panic!("Expected BridgeAttributeAccess");
}

#[test]
fn test_bridge_item_access() {
    let code = r#"
def test(x: Any):
    val = x[10]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();

    if let IrNode::FuncDecl { body, .. } = &ir[0] {
        if let IrNode::VarDecl { init: Some(expr), .. } = &body[0] {
            if let IrExpr::BridgeItemAccess { target: _, index: _ } = expr.as_ref() {
                return;
            }
        }
    }
    panic!("Expected BridgeItemAccess");
}

#[test]
fn test_bridge_slice() {
    let code = r#"
def test(x: Any):
    val = x[1:10:2]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();

    if let IrNode::FuncDecl { body, .. } = &ir[0] {
        if let IrNode::VarDecl { init: Some(expr), .. } = &body[0] {
            if let IrExpr::BridgeSlice { target: _, start, stop, step } = expr.as_ref() {
                assert!(matches!(start.as_ref(), IrExpr::IntLit(1)));
                assert!(matches!(stop.as_ref(), IrExpr::IntLit(10)));
                assert!(matches!(step.as_ref(), IrExpr::IntLit(2)));
                return;
            }
        }
    }
    panic!("Expected BridgeSlice");
}

#[test]
fn test_bridge_slice_none() {
    let code = r#"
def test(x: Any):
    val = x[:]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();

    if let IrNode::FuncDecl { body, .. } = &ir[0] {
        if let IrNode::VarDecl { init: Some(expr), .. } = &body[0] {
            if let IrExpr::BridgeSlice { target: _, start, stop, step } = expr.as_ref() {
                // Using matches! pattern with reference deref if needed
                assert!(matches!(start.as_ref(), IrExpr::NoneLit));
                assert!(matches!(stop.as_ref(), IrExpr::NoneLit));
                assert!(matches!(step.as_ref(), IrExpr::NoneLit));
                return;
            }
        }
    }
    panic!("Expected BridgeSlice with None args");
}
