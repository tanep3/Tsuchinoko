use crate::ir::{IrNode, IrExprKind};
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
            if let IrExprKind::BridgeMethodCall { target: _, method, args, keywords: _ } = &expr.kind {
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
            if let IrExprKind::BridgeAttributeAccess { target: _, attribute } = &expr.kind {
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
            if let IrExprKind::BridgeItemAccess { target: _, index: _ } = &expr.kind {
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
            let inner = match &expr.kind {
                IrExprKind::BridgeSlice { .. } => expr.as_ref(),
                IrExprKind::TnkValueFrom(inner) => inner.as_ref(),
                _ => expr.as_ref(),
            };
            if let IrExprKind::BridgeSlice { target: _, start, stop, step } = &inner.kind {
                assert!(matches!(start.kind, IrExprKind::IntLit(1)));
                assert!(matches!(stop.kind, IrExprKind::IntLit(10)));
                assert!(matches!(step.kind, IrExprKind::IntLit(2)));
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
            let inner = match &expr.kind {
                IrExprKind::BridgeSlice { .. } => expr.as_ref(),
                IrExprKind::TnkValueFrom(inner) => inner.as_ref(),
                _ => expr.as_ref(),
            };
            if let IrExprKind::BridgeSlice { target: _, start, stop, step } = &inner.kind {
                // Using matches! pattern
                assert!(matches!(start.kind, IrExprKind::NoneLit));
                assert!(matches!(stop.kind, IrExprKind::NoneLit));
                assert!(matches!(step.kind, IrExprKind::NoneLit));
                return;
            }
        }
    }
    panic!("Expected BridgeSlice with None args");
}
