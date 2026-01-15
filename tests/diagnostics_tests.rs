use std::path::Path;

use tsuchinoko::diagnostics::{scan_unsupported_syntax, DiagnosticSeverity, TnkDiagnostics};
use tsuchinoko::unsupported_features::UnsupportedFeatureRegistry;

#[test]
fn test_scan_unsupported_syntax_basic_keywords() {
    let source = r#"
match x:
    case 1:
        pass

del x
async def f():
    await g()
"#;
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, Some(Path::new("sample.py")), &registry);
    assert!(diags.diagnostics.len() >= 4);
    assert!(diags.diagnostics.iter().all(|d| matches!(d.severity, DiagnosticSeverity::Error)));
}

#[test]
fn test_scan_unsupported_syntax_ignores_comments_and_strings() {
    let source = r#"
# match should be ignored here
text = "match del async await yield"
value = 'del'
"#;
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert_eq!(diags.diagnostics.len(), 0);
}

#[test]
fn test_scan_unsupported_syntax_detects_walrus() {
    let source = "if (n := 10):\n    pass\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.code == "TNK-UNSUPPORTED-SYNTAX"));
}

#[test]
fn test_scan_unsupported_syntax_magic_method() {
    let source = "class A:\n    def __iter__(self):\n        pass\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("__iter__")));
}

#[test]
fn test_scan_unsupported_syntax_magic_method_getitem() {
    let source = "class A:\n    def __getitem__(self, idx):\n        pass\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("__getitem__")));
}

#[test]
fn test_scan_unsupported_syntax_custom_decorator() {
    let source = "@classmethod\ndef foo(cls):\n    pass\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("classmethod")));
}

#[test]
fn test_scan_unsupported_syntax_builtin_iter_next() {
    let source = "it = iter([1, 2, 3])\nvalue = next(it)\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("iter()")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("next()")));
}

#[test]
fn test_scan_unsupported_syntax_builtin_iter_ignores_attribute_and_binding() {
    let source = "iter = 1\nobj.iter()\nobj.next()\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.is_empty());
}

#[test]
fn test_scan_unsupported_syntax_builtin_reflection() {
    let source = "value = getattr(obj, \"x\")\nsetattr(obj, \"x\", 1)\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("getattr()")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("setattr()")));
}

#[test]
fn test_scan_unsupported_syntax_builtin_reflection_ignores_def_and_attr() {
    let source = "def getattr(x):\n    return x\nobj.getattr()\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.is_empty());
}

#[test]
fn test_scan_unsupported_syntax_builtin_introspection() {
    let source = "items = dir(obj)\nvalues = vars(obj)\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("dir()")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("vars()")));
}

#[test]
fn test_scan_unsupported_syntax_builtin_introspection_ignores_def_and_attr() {
    let source = "def dir(x):\n    return x\nobj.dir()\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.is_empty());
}

#[test]
fn test_scan_unsupported_syntax_builtin_type_checks() {
    let source = "kind = type(obj)\nflag = issubclass(A, B)\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("type()")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("issubclass()")));
}

#[test]
fn test_scan_unsupported_syntax_builtin_type_checks_ignores_def_and_attr() {
    let source = "def type(x):\n    return x\nobj.type()\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.is_empty());
}

#[test]
fn test_scan_unsupported_syntax_builtin_identity() {
    let source = "value = id(obj)\nvalue = hash(obj)\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("id()")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("hash()")));
}

#[test]
fn test_scan_unsupported_syntax_builtin_identity_ignores_def_and_attr() {
    let source = "def id(x):\n    return x\nobj.id()\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.is_empty());
}

#[test]
fn test_scan_unsupported_syntax_builtin_format_repr() {
    let source = "value = format(123, \"d\")\ntext = repr(obj)\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("format()")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("repr()")));
}

#[test]
fn test_scan_unsupported_syntax_builtin_format_repr_ignores_def_and_attr() {
    let source = "def format(x):\n    return x\nobj.format()\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.is_empty());
}

#[test]
fn test_scan_unsupported_ast_generator_expr_is_allowed() {
    let source = "sum(x for x in [1, 2, 3])\n";
    let program = tsuchinoko::parser::parse(source).expect("parse ok");
    let registry = UnsupportedFeatureRegistry::default();
    let diags = tsuchinoko::diagnostics::scan_unsupported_ast(&program, None, &registry);
    assert!(diags.diagnostics.is_empty());
}

#[test]
fn test_scan_unsupported_ast_custom_context_manager() {
    let source = "with ctx():\n    pass\n";
    let program = tsuchinoko::parser::parse(source).expect("parse ok");
    let registry = UnsupportedFeatureRegistry::default();
    let diags = tsuchinoko::diagnostics::scan_unsupported_ast(&program, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("context manager")));
}

#[test]
fn test_scan_unsupported_ast_multiple_inheritance() {
    let source = "class A(B, C):\n    pass\n";
    let program = tsuchinoko::parser::parse(source).expect("parse ok");
    let registry = UnsupportedFeatureRegistry::default();
    let diags = tsuchinoko::diagnostics::scan_unsupported_ast(&program, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("multiple inheritance")));
}

#[test]
fn test_scan_unsupported_syntax_multiple_inheritance() {
    let source = "class A(B, C):\n    pass\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("multiple inheritance")));
}

#[test]
fn test_scan_unsupported_syntax_keywords_global_nonlocal_type_del() {
    let source = "global x\nnonlocal y\ndel z\ntype Alias = int\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("global")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("nonlocal")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("del")));
    assert!(diags.diagnostics.iter().any(|d| d.message.contains("type")));
}

#[test]
fn test_scan_unsupported_syntax_keyword_boundaries() {
    let source = "global_var = 1\nnonlocal_var = 2\ntyped = 3\nmodel = 4\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.diagnostics.is_empty());
}

#[test]
fn test_parse_class_with_pass_body() {
    let source = "class A:\n    pass\n";
    let program = tsuchinoko::parser::parse(source).expect("parse ok");
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_scan_unsupported_ir_match_statement() {
    use tsuchinoko::ir::{IrNode, MatchArm};
    use tsuchinoko::ir::exprs::{IrExpr, IrExprKind, ExprId};
    let ir = vec![IrNode::Match {
        value: IrExpr { id: ExprId(0), kind: IrExprKind::IntLit(1) },
        arms: vec![MatchArm {
            variant: "1".to_string(),
            binding: "v".to_string(),
            body: vec![IrNode::Break],
        }],
    }];
    let registry = UnsupportedFeatureRegistry::default();
    let diags = tsuchinoko::diagnostics::scan_unsupported_ir(&ir, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.code == "TNK-UNSUPPORTED-SYNTAX"));
}

#[test]
fn test_scan_unsupported_ir_magic_method_iter() {
    use tsuchinoko::ir::IrNode;
    let ir = vec![IrNode::MethodDecl {
        name: "__iter__".to_string(),
        params: vec![],
        ret: tsuchinoko::semantic::Type::Unknown,
        body: vec![],
        takes_self: true,
        takes_mut_self: false,
        may_raise: false,
        needs_bridge: false,
    }];
    let registry = UnsupportedFeatureRegistry::default();
    let diags = tsuchinoko::diagnostics::scan_unsupported_ir(&ir, None, &registry);
    assert!(diags.diagnostics.iter().any(|d| d.code == "TNK-UNSUPPORTED-SYNTAX"));
}

#[test]
fn test_diagnostics_to_text_and_json() {
    let source = "del x\n";
    let registry = UnsupportedFeatureRegistry::default();
    let diags = scan_unsupported_syntax(source, None, &registry);
    assert!(diags.has_errors());
    let text = diags.to_text();
    let json = diags.to_json();
    assert!(text.contains("TNK-UNSUPPORTED-SYNTAX"));
    assert!(json.contains("TNK-UNSUPPORTED-SYNTAX"));
    assert!(json.contains("\"diagnostics\""));
}

#[test]
fn test_diagnostics_empty() {
    let diags = TnkDiagnostics::default();
    assert!(!diags.has_errors());
    assert_eq!(diags.to_text(), "");
}
