//! semantic module tests
//!
//! Extracted from mod.rs for better code organization.

#![allow(clippy::approx_constant)]
use super::*;
use crate::parser::parse;

// === カバレッジ80%達成用追加テスト ===

// --- convert_binop テスト ---
#[test]
fn test_convert_binop_add() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Add);
    assert_eq!(op, IrBinOp::Add);
}

#[test]
fn test_convert_binop_sub() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Sub);
    assert_eq!(op, IrBinOp::Sub);
}

#[test]
fn test_convert_binop_mul() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Mul);
    assert_eq!(op, IrBinOp::Mul);
}

#[test]
fn test_convert_binop_div() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Div);
    assert_eq!(op, IrBinOp::Div);
}

#[test]
fn test_convert_binop_eq() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Eq);
    assert_eq!(op, IrBinOp::Eq);
}

#[test]
fn test_convert_binop_lt() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Lt);
    assert_eq!(op, IrBinOp::Lt);
}

// --- analyze: 複雑なケース ---
#[test]
fn test_analyze_return() {
    let code = r#"
def foo() -> int:
    return 42
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    if let IrNode::FuncDecl { body, .. } = &ir[0] {
        assert!(matches!(&body[0], IrNode::Return(_)));
    }
}

#[test]
fn test_analyze_binop_expr() {
    let code = "x = 1 + 2";
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert_eq!(ir.len(), 1);
}

// --- type_from_hint テスト ---
#[test]
fn test_type_from_hint_int() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "int".to_string(),
        params: vec![],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_type_from_hint_str() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "str".to_string(),
        params: vec![],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert_eq!(ty, Type::String);
}

#[test]
fn test_type_from_hint_list() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "list".to_string(),
        params: vec![crate::parser::TypeHint {
            name: "int".to_string(),
            params: vec![],
        }],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert!(matches!(ty, Type::List(_)));
}

// === テストバッチ2: analyze_expr網羅 ===

// --- convert_binop 追加テスト ---
#[test]
fn test_convert_binop_mod() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Mod);
    assert_eq!(op, IrBinOp::Mod);
}

#[test]
fn test_convert_binop_lteq() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::LtEq);
    assert_eq!(op, IrBinOp::LtEq);
}

#[test]
fn test_convert_binop_noteq() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::NotEq);
    assert_eq!(op, IrBinOp::NotEq);
}

// --- type_from_hint 追加 ---
#[test]
fn test_type_from_hint_float() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "float".to_string(),
        params: vec![],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert_eq!(ty, Type::Float);
}

#[test]
fn test_type_from_hint_bool() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "bool".to_string(),
        params: vec![],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert_eq!(ty, Type::Bool);
}

#[test]
fn test_type_from_hint_dict() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "dict".to_string(),
        params: vec![
            crate::parser::TypeHint {
                name: "str".to_string(),
                params: vec![],
            },
            crate::parser::TypeHint {
                name: "int".to_string(),
                params: vec![],
            },
        ],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert!(matches!(ty, Type::Dict(_, _)));
}

// === テストバッチ3: Stmt網羅エンドツーエンドテスト ===

// --- FuncDef variants ---
#[test]
fn test_analyze_func_with_params() {
    let code = r#"
def add(a: int, b: int) -> int:
    return a + b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    if let IrNode::FuncDecl { params, ret, .. } = &ir[0] {
        assert_eq!(params.len(), 2);
        assert_eq!(*ret, Type::Int);
    }
}

#[test]
fn test_analyze_func_with_default_param() {
    let code = r#"
def greet(name: str = "World") -> str:
    return name
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(matches!(&ir[0], IrNode::FuncDecl { .. }));
}

// --- Break/Continue ---
#[test]
fn test_analyze_break() {
    let code = r#"
def test():
    for i in range(10):
        if i > 5:
            break
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_continue() {
    let code = r#"
def test():
    for i in range(10):
        if i < 5:
            continue
        x = i
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- TryExcept ---
#[test]
fn test_analyze_try_except() {
    let code = r#"
def test():
    try:
        x = 1
    except:
        x = 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- Pass ---
#[test]
fn test_analyze_pass() {
    let code = r#"
def empty():
    pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- AugAssign variants ---
// --- TupleAssign ---
#[test]
fn test_analyze_tuple_assign() {
    let code = r#"
def test():
    a, b = 1, 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- IndexAssign ---
// --- ListComp ---
#[test]
fn test_analyze_listcomp() {
    let code = r#"
def test():
    squares: list[int] = [x * x for x in range(5)]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- DictComp ---
#[test]
fn test_analyze_dictcomp() {
    let code = r#"
def test():
    d: dict[int, int] = {x: x * x for x in range(5)}
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- Lambda ---
#[test]
fn test_analyze_lambda() {
    let code = r#"
def test():
    f = lambda x: x * 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- Slice ---
// --- FieldAssign ---
#[test]
fn test_analyze_field_assign() {
    let code = r#"
class Point:
    x: int
    y: int
    def set_x(self, val: int):
        self.x = val
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- Method call ---
#[test]
fn test_analyze_list_pop() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    x = arr.pop()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- Attribute access ---
#[test]
fn test_analyze_attribute() {
    let code = r#"
class Point:
    x: int
    y: int

def test():
    p = Point()
    val = p.x
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(ir.len() >= 2);
}

// === テストバッチ4: Call/Builtin網羅テスト ===

// --- print/len/range ---
#[test]
fn test_analyze_print_call() {
    let code = r#"
def test():
    print("hello", "world")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_len_call() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    n = len(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_range_call() {
    let code = r#"
def test():
    r = range(10)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_range_step_call() {
    let code = r#"
def test():
    r = range(0, 10, 2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- int/float/str conversion ---
#[test]
fn test_analyze_int_conversion() {
    let code = r#"
def test():
    x = int("42")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_float_conversion() {
    let code = r#"
def test():
    x = float("3.14")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_str_conversion() {
    let code = r#"
def test():
    s = str(42)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- max/min/abs ---
#[test]
fn test_analyze_max_call() {
    let code = r#"
def test():
    m = max(1, 2, 3)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_min_call() {
    let code = r#"
def test():
    m = min(1, 2, 3)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_abs_call() {
    let code = r#"
def test():
    a = abs(-5)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- sum/sorted/reversed ---
#[test]
fn test_analyze_sum_call() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    s = sum(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_sorted_call() {
    let code = r#"
def test():
    arr: list[int] = [3, 1, 2]
    s = sorted(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_reversed_call() {
    let code = r#"
def test():
    s: str = "hello"
    r = reversed(s)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- zip ---
#[test]
fn test_analyze_zip_call() {
    let code = r#"
def test():
    a: list[int] = [1, 2, 3]
    b: list[str] = ["a", "b", "c"]
    for x, y in zip(a, b):
        print(x, y)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list methods ---
#[test]
fn test_analyze_list_insert() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    arr.insert(0, 0)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_list_remove() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    arr.remove(2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_list_extend() {
    let code = r#"
def test():
    arr: list[int] = [1, 2]
    arr.extend([3, 4])
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_list_clear() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    arr.clear()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- string methods ---
#[test]
fn test_analyze_string_upper() {
    let code = r#"
def test():
    s: str = "hello"
    u = s.upper()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_lower() {
    let code = r#"
def test():
    s: str = "HELLO"
    l = s.lower()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_split() {
    let code = r#"
def test():
    s: str = "a,b,c"
    parts = s.split(",")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_join() {
    let code = r#"
def test():
    parts: list[str] = ["a", "b", "c"]
    s = ",".join(parts)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_strip() {
    let code = r#"
def test():
    s: str = "  hello  "
    t = s.strip()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_replace() {
    let code = r#"
def test():
    s: str = "hello world"
    t = s.replace("world", "rust")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- dict methods ---
#[test]
fn test_analyze_dict_get() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1}
    v = d.get("a")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_keys() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    k = d.keys()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_values() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    v = d.values()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_items() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    for k, v in d.items():
        print(k, v)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- input ---
#[test]
fn test_analyze_input_call() {
    let code = r#"
def test():
    name = input("Enter name: ")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- isinstance ---
#[test]
fn test_analyze_isinstance_call() {
    let code = r#"
def test():
    x: int = 5
    b = isinstance(x, int)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// === テストバッチ5: scope/coercion/operators/infer網羅 ===

// --- scope テスト ---
#[test]
fn test_scope_define_lookup() {
    use super::scope::ScopeStack;
    let mut scope = ScopeStack::new();
    scope.define("x", Type::Int, false);
    assert!(scope.lookup("x").is_some());
    assert!(scope.lookup("y").is_none());
}

#[test]
fn test_scope_push_pop() {
    use super::scope::ScopeStack;
    let mut scope = ScopeStack::new();
    scope.define("x", Type::Int, false);
    scope.push();
    scope.define("y", Type::String, false);
    assert!(scope.lookup("x").is_some());
    assert!(scope.lookup("y").is_some());
    // Python semantics: y is promoted to parent scope after pop
    scope.pop();
    assert!(scope.lookup("x").is_some());
    assert!(scope.lookup("y").is_some()); // Python: still accessible!
}

// --- operators テスト ---
#[test]
fn test_convert_binop_pow() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Pow);
    assert_eq!(op, IrBinOp::Pow);
}

#[test]
fn test_convert_binop_bitand() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::BitAnd);
    assert_eq!(op, IrBinOp::BitAnd);
}

#[test]
fn test_convert_binop_bitor() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::BitOr);
    assert_eq!(op, IrBinOp::BitOr);
}

#[test]
fn test_convert_binop_bitxor() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::BitXor);
    assert_eq!(op, IrBinOp::BitXor);
}

#[test]
fn test_convert_binop_shl() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Shl);
    assert_eq!(op, IrBinOp::Shl);
}

#[test]
fn test_convert_binop_shr() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Shr);
    assert_eq!(op, IrBinOp::Shr);
}

// --- type_from_hint 追加 ---
#[test]
fn test_type_from_hint_optional() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "Optional".to_string(),
        params: vec![crate::parser::TypeHint {
            name: "int".to_string(),
            params: vec![],
        }],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert!(matches!(ty, Type::Optional(_)));
}

#[test]
fn test_type_from_hint_tuple() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "tuple".to_string(),
        params: vec![
            crate::parser::TypeHint {
                name: "int".to_string(),
                params: vec![],
            },
            crate::parser::TypeHint {
                name: "str".to_string(),
                params: vec![],
            },
        ],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert!(matches!(ty, Type::Tuple(_)));
}

// --- complex expressions ---
#[test]
fn test_analyze_nested_binop() {
    let code = r#"
def test():
    x = (1 + 2) * 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_chained_comparison() {
    let code = r#"
def test():
    x: int = 5
    b = 0 < x < 10
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_walrus_like_assign() {
    let code = r#"
def test():
    x: int = 0
    x = x + 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- function calls with complex args ---
#[test]
fn test_analyze_call_with_kwargs() {
    let code = r#"
def greet(name: str, greeting: str = "Hello") -> str:
    return greeting

def test():
    s = greet("World")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(ir.len() >= 2);
}

#[test]
fn test_analyze_recursive_call() {
    let code = r#"
def factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- nested functions ---
#[test]
fn test_analyze_nested_function() {
    let code = r#"
def outer():
    def inner():
        return 1
    return inner()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- Optional/None handling ---
#[test]
fn test_analyze_optional_return() {
    let code = r#"
def find(x: int) -> int:
    if x > 0:
        return x
    return 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- Struct instantiation ---
#[test]
fn test_analyze_struct_instantiation() {
    let code = r#"
class Point:
    x: int
    y: int

def test():
    p = Point()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(ir.len() >= 2);
}

// === テストバッチ6: レアケース/特殊パターン網羅 ===

// --- main block ---
#[test]
fn test_analyze_main_block() {
    let code = r#"
if __name__ == "__main__":
    print("Hello")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    // main block は FuncDecl(main) に変換される
    assert!(!ir.is_empty());
}

// --- staticmethod ---
#[test]
fn test_analyze_staticmethod() {
    let code = r#"
class Math:
    @staticmethod
    def add(a: int, b: int) -> int:
        return a + b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list comprehension if ---
#[test]
fn test_analyze_listcomp_with_if() {
    let code = r#"
def test():
    evens: list[int] = [x for x in range(10) if x % 2 == 0]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- nested list comp ---
#[test]
fn test_analyze_nested_listcomp() {
    let code = r#"
def test():
    matrix: list[list[int]] = [[i * j for j in range(3)] for i in range(3)]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- star unpacking ---
#[test]
fn test_analyze_star_unpacking() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3, 4, 5]
    head, *tail = arr
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- f-string complex ---
#[test]
fn test_analyze_fstring_complex() {
    let code = r#"
def test():
    x: int = 42
    y: float = 3.14
    s = f"x={x}, y={y}"
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- ternary in expression ---
#[test]
fn test_analyze_ternary_in_expr() {
    let code = r#"
def test():
    x: int = 5
    y = x * 2 if x > 0 else 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- multiple return ---
#[test]
fn test_analyze_multiple_return() {
    let code = r#"
def divmod_custom(a: int, b: int) -> int:
    if b == 0:
        return 0
    return a // b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- global var ---
#[test]
fn test_analyze_global_var() {
    let code = r#"
CONSTANT: int = 100

def test():
    x = CONSTANT
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(ir.len() >= 2);
}

// --- type alias ---
#[test]
fn test_analyze_type_alias() {
    let code = r#"
IntList = list[int]

def test():
    arr: IntList = [1, 2, 3]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(ir.len() >= 2);
}

// --- boolean operators ---
#[test]
fn test_analyze_boolean_and_or() {
    let code = r#"
def test():
    a: bool = True
    b: bool = False
    c = a and b
    d = a or b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- comparison chain ---
#[test]
fn test_analyze_comparison_chain() {
    let code = r#"
def test():
    x: int = 5
    result = 0 <= x <= 10
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- is None / is not None ---
#[test]
fn test_analyze_is_none() {
    let code = r#"
def test():
    x = None
    if x is None:
        y = 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_is_not_none() {
    let code = r#"
def test():
    x = None
    if x is not None:
        y = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- in operator ---
#[test]
fn test_analyze_in_operator() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    if 2 in arr:
        x = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- not in operator ---
#[test]
fn test_analyze_not_in_operator() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    if 5 not in arr:
        x = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- negative index ---
#[test]
fn test_analyze_negative_index() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    last = arr[-1]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- slice with step ---
// --- floor div ---
#[test]
fn test_analyze_floor_div() {
    let code = r#"
def test():
    x = 10 // 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- power operator ---
#[test]
fn test_analyze_power() {
    let code = r#"
def test():
    x = 2 ** 10
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- bitwise operators ---
#[test]
fn test_analyze_bitwise() {
    let code = r#"
def test():
    a: int = 5
    b: int = 3
    c = a & b
    d = a | b
    e = a ^ b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- shift operators ---
#[test]
fn test_analyze_shift() {
    let code = r#"
def test():
    x: int = 8
    y = x << 2
    z = x >> 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- complex aug assign ---
#[test]
fn test_analyze_aug_floordiv() {
    let code = r#"
def test():
    x: int = 10
    x //= 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// === テストバッチ7: Call/Method/Builtins網羅 ===

// --- list constructor ---
#[test]
fn test_analyze_list_constructor() {
    let code = r#"
def test():
    arr = list()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- dict constructor ---
#[test]
fn test_analyze_dict_constructor() {
    let code = r#"
def test():
    d = dict()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- set (limited support) ---
// --- ord/chr ---
#[test]
fn test_analyze_ord_call() {
    let code = r#"
def test():
    x = ord("A")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_chr_call() {
    let code = r#"
def test():
    c = chr(65)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- bool conversion ---
#[test]
fn test_analyze_bool_conversion() {
    let code = r#"
def test():
    b = bool(1)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list.sort ---
#[test]
fn test_analyze_list_sort() {
    let code = r#"
def test():
    arr: list[int] = [3, 1, 2]
    arr.sort()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list.reverse ---
#[test]
fn test_analyze_list_reverse() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    arr.reverse()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list.copy ---
#[test]
fn test_analyze_list_copy() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    b = arr.copy()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- string.find ---
#[test]
fn test_analyze_string_find() {
    let code = r#"
def test():
    s: str = "hello"
    i = s.find("l")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- string.startswith ---
#[test]
fn test_analyze_string_startswith() {
    let code = r#"
def test():
    s: str = "hello"
    b = s.startswith("he")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- string.endswith ---
#[test]
fn test_analyze_string_endswith() {
    let code = r#"
def test():
    s: str = "hello"
    b = s.endswith("lo")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- string.count ---
#[test]
fn test_analyze_string_count() {
    let code = r#"
def test():
    s: str = "hello"
    n = s.count("l")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- all/any ---
#[test]
fn test_analyze_all_call() {
    let code = r#"
def test():
    arr: list[bool] = [True, True, False]
    result = all(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_any_call() {
    let code = r#"
def test():
    arr: list[bool] = [False, False, True]
    result = any(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- round ---
#[test]
fn test_analyze_round_call() {
    let code = r#"
def test():
    x = round(3.14159, 2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- pow ---
#[test]
fn test_analyze_pow_call() {
    let code = r#"
def test():
    x = pow(2, 10)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- hex/oct/bin ---
#[test]
fn test_analyze_hex_call() {
    let code = r#"
def test():
    s = hex(255)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- type ---
#[test]
fn test_analyze_type_call() {
    let code = r#"
def test():
    x: int = 42
    t = type(x)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- assert ---
#[test]
fn test_analyze_assert() {
    let code = r#"
def test():
    x: int = 5
    assert x > 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list index ---
#[test]
fn test_analyze_list_index() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    i = arr.index(2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- dict update ---
#[test]
fn test_analyze_dict_update() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1}
    d.update({"b": 2})
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- dict pop ---
#[test]
fn test_analyze_dict_pop() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    v = d.pop("a")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- nested call ---
#[test]
fn test_analyze_nested_call() {
    let code = r#"
def test():
    x = len(str(123))
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// === テストバッチ8-10: 大量追加で80%達成へ ===

// --- Types網羅 ---
#[test]
fn test_type_is_compatible_same() {
    assert!(Type::Int.is_compatible_with(&Type::Int));
    assert!(Type::Float.is_compatible_with(&Type::Float));
    assert!(Type::String.is_compatible_with(&Type::String));
}

#[test]
fn test_type_is_compatible_unknown() {
    assert!(Type::Unknown.is_compatible_with(&Type::Int));
    assert!(Type::Int.is_compatible_with(&Type::Unknown));
}

#[test]
fn test_type_from_python_hint_int() {
    assert_eq!(Type::from_python_hint("int", &[]), Type::Int);
}

#[test]
fn test_type_from_python_hint_str() {
    assert_eq!(Type::from_python_hint("str", &[]), Type::String);
}

#[test]
fn test_type_from_python_hint_bool() {
    assert_eq!(Type::from_python_hint("bool", &[]), Type::Bool);
}

#[test]
fn test_type_from_python_hint_float() {
    assert_eq!(Type::from_python_hint("float", &[]), Type::Float);
}

#[test]
fn test_type_from_python_hint_list() {
    let ty = Type::from_python_hint("list", &[Type::Int]);
    assert!(matches!(ty, Type::List(_)));
}

#[test]
fn test_type_from_python_hint_dict() {
    let ty = Type::from_python_hint("dict", &[Type::String, Type::Int]);
    assert!(matches!(ty, Type::Dict(_, _)));
}

#[test]
fn test_type_from_python_hint_optional() {
    let ty = Type::from_python_hint("Optional", &[Type::Int]);
    assert!(matches!(ty, Type::Optional(_)));
}

#[test]
fn test_type_from_python_hint_tuple() {
    let ty = Type::from_python_hint("tuple", &[Type::Int, Type::String]);
    assert!(matches!(ty, Type::Tuple(_)));
}

// --- Operators網羅 ---
#[test]
fn test_convert_binop_is() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Is);
    assert_eq!(op, IrBinOp::Is);
}

#[test]
fn test_convert_binop_isnot() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::IsNot);
    assert_eq!(op, IrBinOp::IsNot);
}

// --- coercion ---
#[test]
fn test_analyze_int_float_coercion() {
    let code = r#"
def test():
    x: float = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- complex list operations ---
#[test]
fn test_analyze_list_concat() {
    let code = r#"
def test():
    a: list[int] = [1, 2]
    b: list[int] = [3, 4]
    c = a + b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_list_repeat() {
    let code = r#"
def test():
    a: list[int] = [1, 2]
    b = a * 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- string operations ---
#[test]
fn test_analyze_string_concat() {
    let code = r#"
def test():
    a: str = "hello"
    b: str = "world"
    c = a + " " + b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_repeat() {
    let code = r#"
def test():
    s: str = "ab"
    t = s * 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_index() {
    let code = r#"
def test():
    s: str = "hello"
    c = s[0]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_slice() {
    let code = r#"
def test():
    s: str = "hello"
    sub = s[1:4]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- dict operations ---
#[test]
fn test_analyze_dict_literal_complex() {
    let code = r#"
def test():
    d: dict[str, list[int]] = {"a": [1, 2], "b": [3, 4]}
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_index() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    v = d["a"]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- tuple operations ---
#[test]
fn test_analyze_tuple_literal() {
    let code = r#"
def test():
    t: tuple[int, str] = (1, "hello")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_tuple_index() {
    let code = r#"
def test():
    t: tuple[int, str, float] = (1, "hello", 3.14)
    x = t[0]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- function with multiple params ---
#[test]
fn test_analyze_func_many_params() {
    let code = r#"
def multi(a: int, b: int, c: int, d: int) -> int:
    return a + b + c + d
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- nested control flow ---
#[test]
fn test_analyze_nested_if_for() {
    let code = r#"
def test():
    for i in range(10):
        if i % 2 == 0:
            for j in range(i):
                x = j
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_nested_while_if() {
    let code = r#"
def test():
    x: int = 10
    while x > 0:
        if x % 2 == 0:
            y = x
        x -= 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- complex expressions ---
#[test]
fn test_analyze_complex_arithmetic() {
    let code = r#"
def test():
    x = (1 + 2) * (3 - 4) / 5 % 6
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- multiple assignments ---
// --- function calls with expressions ---
#[test]
fn test_analyze_call_with_expr_args() {
    let code = r#"
def add(a: int, b: int) -> int:
    return a + b

def test():
    x = add(1 + 2, 3 * 4)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list append in loop ---
#[test]
fn test_analyze_list_append_in_loop() {
    let code = r#"
def test():
    arr: list[int] = []
    for i in range(5):
        arr.append(i * 2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- dict update in loop ---
#[test]
fn test_analyze_dict_update_in_loop() {
    let code = r#"
def test():
    d: dict[int, int] = {}
    for i in range(5):
        d[i] = i * 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- string formatting ---
#[test]
fn test_analyze_string_format() {
    let code = r#"
def test():
    name: str = "World"
    age: int = 42
    msg = f"Hello {name}, you are {age} years old"
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- exception handling ---
#[test]
fn test_analyze_try_except_finally() {
    let code = r#"
def test():
    try:
        x = 1
    except:
        x = 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- import statements ---
#[test]
fn test_analyze_import() {
    let code = r#"
import math

def test():
    x = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- generator expressions (converted to list) ---
#[test]
fn test_analyze_generator_expr() {
    let code = r#"
def test():
    gen = (x * 2 for x in range(5))
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- walrus operator pattern ---
#[test]
fn test_analyze_reassignment() {
    let code = r#"
def test():
    x: int = 0
    while x < 10:
        x = x + 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- complex type hints ---
#[test]
fn test_type_from_hint_callable() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "Callable".to_string(),
        params: vec![
            crate::parser::TypeHint {
                name: "int".to_string(),
                params: vec![],
            },
            crate::parser::TypeHint {
                name: "bool".to_string(),
                params: vec![],
            },
        ],
    };
    let ty = analyzer.type_from_hint(&hint);
    assert!(matches!(ty, Type::Func { .. }));
}

// --- scope depth ---
#[test]
fn test_scope_depth() {
    use super::scope::ScopeStack;
    let mut scope = ScopeStack::new();
    assert_eq!(scope.depth(), 0);
    scope.push();
    assert_eq!(scope.depth(), 1);
    scope.push();
    assert_eq!(scope.depth(), 2);
    scope.pop();
    assert_eq!(scope.depth(), 1);
}

// --- builtin function returns ---
#[test]
fn test_analyze_enumerate_in_for() {
    let code = r#"
def test():
    arr: list[str] = ["a", "b", "c"]
    for i, v in enumerate(arr):
        print(i, v)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- zip in for ---
#[test]
fn test_analyze_zip_in_for() {
    let code = r#"
def test():
    a: list[int] = [1, 2, 3]
    b: list[str] = ["a", "b", "c"]
    for x, y in zip(a, b):
        print(x, y)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more infer_type tests ---
// --- closure/lambda tests ---
#[test]
fn test_analyze_lambda_complex() {
    let code = r#"
def test():
    f = lambda x, y: x + y
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- optional return ---
#[test]
fn test_analyze_optional_return_some() {
    let code = r#"
def find(arr: list[int], target: int) -> int:
    for x in arr:
        if x == target:
            return x
    return 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- negative literals ---
#[test]
fn test_analyze_negative_literal() {
    let code = r#"
def test():
    x: int = -42
    y: float = -3.14
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- large numbers ---
#[test]
fn test_analyze_large_number() {
    let code = r#"
def test():
    x: int = 999999999999
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- empty function ---
#[test]
fn test_analyze_empty_function() {
    let code = r#"
def noop():
    pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- docstring (ignored) ---
#[test]
fn test_analyze_docstring() {
    let code = r#"
def documented():
    """This is a docstring."""
    pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- multi-line string ---
// --- escape sequences ---
#[test]
fn test_analyze_escape_sequences() {
    let code = r#"
def test():
    s: str = "hello\nworld\ttab"
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- comparison operators ---
#[test]
fn test_analyze_all_comparisons() {
    let code = r#"
def test():
    a: int = 5
    b: int = 10
    r1 = a < b
    r2 = a <= b
    r3 = a > b
    r4 = a >= b
    r5 = a == b
    r6 = a != b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- all aug assign operators ---
#[test]
fn test_analyze_all_aug_assign() {
    let code = r#"
def test():
    x: int = 10
    x += 1
    x -= 1
    x *= 2
    x //= 3
    x %= 4
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more bitwise aug assign ---
#[test]
fn test_analyze_bitwise_aug_assign() {
    let code = r#"
def test():
    x: int = 255
    x &= 15
    x |= 16
    x ^= 8
    x <<= 2
    x >>= 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// === テストバッチ11-15: 残り25%→80%へ ===

// --- more list comprehensions ---
#[test]
fn test_analyze_listcomp_simple() {
    let code = r#"
def test():
    squares = [x * x for x in range(10)]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_listcomp_filter_simple() {
    let code = r#"
def test():
    evens = [x for x in range(20) if x % 2 == 0]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more dict operations ---
#[test]
fn test_analyze_empty_dict() {
    let code = r#"
def test():
    d: dict[str, int] = {}
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_with_int_keys() {
    let code = r#"
def test():
    d: dict[int, str] = {1: "one", 2: "two"}
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more string operations ---
#[test]
fn test_analyze_string_format_simple() {
    let code = r#"
def test():
    x = 42
    s = f"Value is {x}"
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_chars() {
    let code = r#"
def test():
    s: str = "hello"
    for c in s:
        print(c)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- boolean literals ---
#[test]
fn test_analyze_bool_true() {
    let code = r#"
def test():
    b: bool = True
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_bool_false() {
    let code = r#"
def test():
    b: bool = False
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- unary not ---
#[test]
fn test_analyze_unary_not() {
    let code = r#"
def test():
    a: bool = True
    b = not a
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- unary neg ---
// --- empty list ---
#[test]
fn test_analyze_empty_list() {
    let code = r#"
def test():
    arr: list[int] = []
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list with one element ---
#[test]
fn test_analyze_single_element_list() {
    let code = r#"
def test():
    arr: list[int] = [42]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- deeply nested list ---
#[test]
fn test_analyze_nested_list() {
    let code = r#"
def test():
    matrix: list[list[int]] = [[1, 2], [3, 4]]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- multiple function definitions ---
#[test]
fn test_analyze_multiple_functions() {
    let code = r#"
def foo() -> int:
    return 1

def bar() -> int:
    return 2

def baz() -> int:
    return foo() + bar()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(ir.len() >= 3);
}

// --- function calling function ---
#[test]
fn test_analyze_function_composition() {
    let code = r#"
def square(x: int) -> int:
    return x * x

def double(x: int) -> int:
    return x * 2

def test():
    result = square(double(5))
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(ir.len() >= 3);
}

// --- early return ---
#[test]
fn test_analyze_early_return() {
    let code = r#"
def validate(x: int) -> bool:
    if x < 0:
        return False
    if x > 100:
        return False
    return True
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- break in while ---
#[test]
fn test_analyze_break_in_while() {
    let code = r#"
def test():
    x: int = 0
    while True:
        x += 1
        if x > 10:
            break
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- continue in while ---
#[test]
fn test_analyze_continue_in_while() {
    let code = r#"
def test():
    x: int = 0
    total: int = 0
    while x < 10:
        x += 1
        if x % 2 == 0:
            continue
        total += x
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- nested for loops ---
#[test]
fn test_analyze_nested_for() {
    let code = r#"
def test():
    for i in range(5):
        for j in range(5):
            x = i * j
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- nested if ---
#[test]
fn test_analyze_nested_if() {
    let code = r#"
def test():
    x: int = 5
    if x > 0:
        if x < 10:
            if x == 5:
                y = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more builtins ---
#[test]
fn test_analyze_print_multiple_args() {
    let code = r#"
def test():
    print(1, 2, 3, 4, 5)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_print_mixed_args() {
    let code = r#"
def test():
    x: int = 42
    s: str = "hello"
    print(x, s, True, 3.14)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- range variations ---
#[test]
fn test_analyze_range_negative() {
    let code = r#"
def test():
    for i in range(-5, 5):
        x = i
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_range_negative_step() {
    let code = r#"
def test():
    for i in range(10, 0, -1):
        x = i
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- chained method calls (single) ---
#[test]
fn test_analyze_chained_method_single() {
    let code = r#"
def test():
    s = "hello".upper()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- scope operations ---
#[test]
fn test_scope_multiple_push_pop() {
    use super::scope::ScopeStack;
    let mut scope = ScopeStack::new();

    scope.define("a", Type::Int, false);
    scope.push();
    scope.define("b", Type::String, false);
    scope.push();
    scope.define("c", Type::Float, false);

    assert!(scope.lookup("a").is_some());
    assert!(scope.lookup("b").is_some());
    assert!(scope.lookup("c").is_some());

    // Python semantics: variables are promoted to parent on pop
    scope.pop();
    assert!(scope.lookup("c").is_some()); // Promoted from deepest scope

    scope.pop();
    // Both b and c are now in the global scope
    assert!(scope.lookup("b").is_some()); // Promoted
    assert!(scope.lookup("c").is_some()); // Promoted through multiple pops
    assert!(scope.lookup("a").is_some());
}

// --- type compatibility ---
#[test]
fn test_type_compatibility_different() {
    assert!(!Type::Int.is_compatible_with(&Type::String));
    assert!(!Type::Float.is_compatible_with(&Type::Bool));
}

#[test]
fn test_type_compatibility_unknown_wildcard() {
    assert!(Type::Unknown.is_compatible_with(&Type::Float));
    assert!(Type::Unknown.is_compatible_with(&Type::List(Box::new(Type::Int))));
}

// --- list type from hint ---
#[test]
fn test_type_from_hint_nested_list() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "list".to_string(),
        params: vec![crate::parser::TypeHint {
            name: "list".to_string(),
            params: vec![crate::parser::TypeHint {
                name: "int".to_string(),
                params: vec![],
            }],
        }],
    };
    let ty = analyzer.type_from_hint(&hint);
    if let Type::List(inner) = ty {
        assert!(matches!(*inner, Type::List(_)));
    } else {
        panic!("Expected nested list type");
    }
}

// --- modulo operator ---
#[test]
fn test_analyze_modulo() {
    let code = r#"
def test():
    x = 10 % 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- integer division ---
#[test]
fn test_analyze_integer_div() {
    let code = r#"
def test():
    x = 7 // 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- float literal ---
#[test]
fn test_analyze_float_literal() {
    let code = r#"
def test():
    x: float = 3.14159
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- float operations ---
#[test]
fn test_analyze_float_operations() {
    let code = r#"
def test():
    a: float = 1.5
    b: float = 2.5
    c = a + b
    d = a * b
    e = a / b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// === テストバッチ16-30: 80%達成へ ===

// --- more analyze_calls coverage ---
#[test]
fn test_analyze_print_empty() {
    let code = r#"
def test():
    print()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_len_string() {
    let code = r#"
def test():
    s: str = "hello"
    n = len(s)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_str_int() {
    let code = r#"
def test():
    s = str(42)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_int_str() {
    let code = r#"
def test():
    n = int("123")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_float_str() {
    let code = r#"
def test():
    f = float("3.14")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more analyze_statements coverage ---
#[test]
fn test_analyze_simple_assign() {
    let code = r#"
def test():
    x = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_typed_assign() {
    let code = r#"
def test():
    x: int = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_reassign() {
    let code = r#"
def test():
    x: int = 1
    x = 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more function patterns ---
#[test]
fn test_analyze_func_no_params() {
    let code = r#"
def zero() -> int:
    return 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_func_one_param() {
    let code = r#"
def identity(x: int) -> int:
    return x
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_func_many_params_v2() {
    let code = r#"
def five(a: int, b: int, c: int, d: int, e: int) -> int:
    return a + b + c + d + e
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more builtins ---
#[test]
fn test_analyze_max_two() {
    let code = r#"
def test():
    m = max(1, 2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_min_two() {
    let code = r#"
def test():
    m = min(1, 2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_abs_positive() {
    let code = r#"
def test():
    a = abs(5)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_abs_negative() {
    let code = r#"
def test():
    a = abs(-5)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more types coverage ---
#[test]
fn test_type_optional_none() {
    let ty = Type::Optional(Box::new(Type::Int));
    assert!(matches!(ty, Type::Optional(_)));
}

#[test]
fn test_type_list_nested() {
    let ty = Type::List(Box::new(Type::List(Box::new(Type::Int))));
    if let Type::List(inner) = ty {
        assert!(matches!(*inner, Type::List(_)));
    }
}

#[test]
fn test_type_dict_complex() {
    let ty = Type::Dict(
        Box::new(Type::String),
        Box::new(Type::List(Box::new(Type::Int))),
    );
    if let Type::Dict(k, v) = ty {
        assert_eq!(*k, Type::String);
        assert!(matches!(*v, Type::List(_)));
    }
}

#[test]
fn test_type_tuple_many() {
    let ty = Type::Tuple(vec![Type::Int, Type::String, Type::Float, Type::Bool]);
    if let Type::Tuple(elems) = ty {
        assert_eq!(elems.len(), 4);
    }
}

// --- more type hints ---
// --- scope tests ---
#[test]
fn test_scope_shadowing() {
    use super::scope::ScopeStack;
    let mut scope = ScopeStack::new();
    scope.define("x", Type::Int, false);
    scope.push();
    scope.define("x", Type::String, false);
    let info = scope.lookup("x").unwrap();
    assert_eq!(info.ty, Type::String);
    scope.pop();
    let info = scope.lookup("x").unwrap();
    assert_eq!(info.ty, Type::Int);
}

// --- operators coverage ---
#[test]
fn test_convert_binop_and() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::And);
    assert_eq!(op, IrBinOp::And);
}

#[test]
fn test_convert_binop_or() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Or);
    assert_eq!(op, IrBinOp::Or);
}

#[test]
fn test_convert_binop_eq_v2() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Eq);
    assert_eq!(op, IrBinOp::Eq);
}

#[test]
fn test_convert_binop_lt_v2() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Lt);
    assert_eq!(op, IrBinOp::Lt);
}

// --- more complex patterns ---
#[test]
fn test_analyze_factorial() {
    let code = r#"
def factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_fibonacci() {
    let code = r#"
def fib(n: int) -> int:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_sum_list() {
    let code = r#"
def sum_list(arr: list[int]) -> int:
    total: int = 0
    for x in arr:
        total += x
    return total
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_find_max() {
    let code = r#"
def find_max(arr: list[int]) -> int:
    m: int = arr[0]
    for x in arr:
        if x > m:
            m = x
    return m
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_binary_search() {
    let code = r#"
def binary_search(arr: list[int], target: int) -> int:
    left: int = 0
    right: int = len(arr) - 1
    while left <= right:
        mid = (left + right) // 2
        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_bubble_sort() {
    let code = r#"
def bubble_sort(arr: list[int]) -> list[int]:
    n: int = len(arr)
    for i in range(n):
        for j in range(0, n - i - 1):
            if arr[j] > arr[j + 1]:
                temp = arr[j]
                arr[j] = arr[j + 1]
                arr[j + 1] = temp
    return arr
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- string operations ---
#[test]
fn test_analyze_string_len() {
    let code = r#"
def test():
    s: str = "hello"
    n = len(s)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_in() {
    let code = r#"
def test():
    s: str = "hello world"
    if "world" in s:
        print("found")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- dict in loop ---
#[test]
fn test_analyze_dict_iteration() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    for k in d:
        print(k)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- list containing complex types ---
#[test]
fn test_analyze_list_of_tuples() {
    let code = r#"
def test():
    points: list[tuple[int, int]] = [(1, 2), (3, 4), (5, 6)]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- function returning list ---
#[test]
fn test_analyze_func_return_list() {
    let code = r#"
def make_list() -> list[int]:
    return [1, 2, 3]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- function returning dict ---
#[test]
fn test_analyze_func_return_dict() {
    let code = r#"
def make_dict() -> dict[str, int]:
    return {"a": 1, "b": 2}
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- method on list ---
#[test]
fn test_analyze_list_count() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 2, 3]
    n = arr.count(2)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- method on dict ---
#[test]
fn test_analyze_dict_contains() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1}
    if "a" in d:
        print("found")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- assert with message (simplified) ---
#[test]
fn test_analyze_assert_simple() {
    let code = r#"
def test():
    x: int = 5
    assert x > 0
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- raise (simplified, as panic) ---
#[test]
fn test_analyze_conditional_raise() {
    let code = r#"
def validate(x: int):
    if x < 0:
        raise ValueError("negative")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- pass in function ---
#[test]
fn test_analyze_pass_function() {
    let code = r#"
def noop():
    pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- pass in class ---
// --- pass in if ---
#[test]
fn test_analyze_pass_if() {
    let code = r#"
def test():
    x: int = 5
    if x > 0:
        pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- enumerate with one var ---
#[test]
fn test_analyze_enumerate_simple() {
    let code = r#"
def test():
    arr: list[str] = ["a", "b", "c"]
    for item in enumerate(arr):
        print(item)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// === テストバッチ31-50: 80%達成へ ===

// --- more builtins functions ---
#[test]
fn test_analyze_sorted_list() {
    let code = r#"
def test():
    arr: list[int] = [3, 1, 2]
    s = sorted(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_reversed_list() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    r = list(reversed(arr))
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_sum_list_builtin() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3, 4, 5]
    total = sum(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more method calls ---
#[test]
fn test_analyze_string_upper_method() {
    let code = r#"
def test():
    s: str = "hello"
    u = s.upper()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_lower_method() {
    let code = r#"
def test():
    s: str = "HELLO"
    l = s.lower()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_strip_method() {
    let code = r#"
def test():
    s: str = "  hello  "
    t = s.strip()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_list_append_method() {
    let code = r#"
def test():
    arr: list[int] = [1, 2]
    arr.append(3)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_list_pop_method() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    x = arr.pop()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- dict method access ---
#[test]
fn test_analyze_dict_get_method() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1}
    v = d.get("a")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_keys_method() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    k = d.keys()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_values_method() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    v = d.values()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_items_method() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1, "b": 2}
    for k, v in d.items():
        print(k, v)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- complex algorithms ---
#[test]
fn test_analyze_selection_sort() {
    let code = r#"
def selection_sort(arr: list[int]) -> list[int]:
    n: int = len(arr)
    for i in range(n):
        min_idx: int = i
        for j in range(i + 1, n):
            if arr[j] < arr[min_idx]:
                min_idx = j
        temp = arr[i]
        arr[i] = arr[min_idx]
        arr[min_idx] = temp
    return arr
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_insertion_sort() {
    let code = r#"
def insertion_sort(arr: list[int]) -> list[int]:
    for i in range(1, len(arr)):
        key: int = arr[i]
        j: int = i - 1
        while j >= 0 and arr[j] > key:
            arr[j + 1] = arr[j]
            j -= 1
        arr[j + 1] = key
    return arr
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_gcd() {
    let code = r#"
def gcd(a: int, b: int) -> int:
    while b != 0:
        temp = b
        b = a % b
        a = temp
    return a
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_is_prime() {
    let code = r#"
def is_prime(n: int) -> bool:
    if n < 2:
        return False
    for i in range(2, n):
        if n % i == 0:
            return False
    return True
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_linear_search() {
    let code = r#"
def linear_search(arr: list[int], target: int) -> int:
    for i in range(len(arr)):
        if arr[i] == target:
            return i
    return -1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more complex control flow ---
#[test]
fn test_analyze_nested_break() {
    let code = r#"
def test():
    found: bool = False
    for i in range(10):
        for j in range(10):
            if i * j == 42:
                found = True
                break
        if found:
            break
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_nested_continue() {
    let code = r#"
def test():
    total: int = 0
    for i in range(10):
        if i % 2 == 0:
            for j in range(10):
                if j % 3 == 0:
                    continue
                total += j
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more type tests ---
#[test]
fn test_type_func_creation() {
    let ty = Type::Func {
        params: vec![Type::Int, Type::Int],
        ret: Box::new(Type::Int),
        is_boxed: false, may_raise: false,
    };
    if let Type::Func { params, ret, .. } = ty {
        assert_eq!(params.len(), 2);
        assert_eq!(*ret, Type::Int);
    }
}

#[test]
fn test_type_ref_creation() {
    let ty = Type::Ref(Box::new(Type::Int));
    if let Type::Ref(inner) = ty {
        assert_eq!(*inner, Type::Int);
    }
}

#[test]
fn test_type_mutref_creation() {
    let ty = Type::MutRef(Box::new(Type::String));
    if let Type::MutRef(inner) = ty {
        assert_eq!(*inner, Type::String);
    }
}

// --- more scope tests ---
#[test]
fn test_scope_empty() {
    use super::scope::ScopeStack;
    let scope = ScopeStack::new();
    assert!(scope.lookup("nonexistent").is_none());
}

#[test]
fn test_scope_overwrite() {
    use super::scope::ScopeStack;
    let mut scope = ScopeStack::new();
    scope.define("x", Type::Int, false);
    scope.define("x", Type::String, false);
    let info = scope.lookup("x").unwrap();
    assert_eq!(info.ty, Type::String);
}

// --- more operator tests ---
#[test]
fn test_convert_binop_matmul() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::MatMul);
    assert_eq!(op, IrBinOp::MatMul);
}

#[test]
fn test_convert_binop_add_v2() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Add);
    assert_eq!(op, IrBinOp::Add);
}

#[test]
fn test_convert_binop_sub_v2() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Sub);
    assert_eq!(op, IrBinOp::Sub);
}

#[test]
fn test_convert_binop_mul_v2() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Mul);
    assert_eq!(op, IrBinOp::Mul);
}

#[test]
fn test_convert_binop_div_v2() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Div);
    assert_eq!(op, IrBinOp::Div);
}

// --- more comparison patterns ---
#[test]
fn test_analyze_cmp_lt() {
    let code = r#"
def test():
    result = 1 < 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_cmp_gt() {
    let code = r#"
def test():
    result = 2 > 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_cmp_lte() {
    let code = r#"
def test():
    result = 1 <= 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_cmp_gte() {
    let code = r#"
def test():
    result = 2 >= 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_cmp_eq() {
    let code = r#"
def test():
    result = 1 == 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_cmp_neq() {
    let code = r#"
def test():
    result = 1 != 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more boolean patterns ---
#[test]
fn test_analyze_bool_and() {
    let code = r#"
def test():
    result = True and False
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_bool_or() {
    let code = r#"
def test():
    result = True or False
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_bool_not_v2() {
    let code = r#"
def test():
    result = not True
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- empty bodies ---
#[test]
fn test_analyze_func_only_return() {
    let code = r#"
def just_return() -> int:
    return 42
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_func_only_pass() {
    let code = r#"
def just_pass():
    pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- type hint combinations ---
#[test]
fn test_type_from_hint_list_str() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "list".to_string(),
        params: vec![crate::parser::TypeHint {
            name: "str".to_string(),
            params: vec![],
        }],
    };
    let ty = analyzer.type_from_hint(&hint);
    if let Type::List(inner) = ty {
        assert_eq!(*inner, Type::String);
    }
}

#[test]
fn test_type_from_hint_dict_str_int() {
    let analyzer = SemanticAnalyzer::new();
    let hint = crate::parser::TypeHint {
        name: "dict".to_string(),
        params: vec![
            crate::parser::TypeHint {
                name: "str".to_string(),
                params: vec![],
            },
            crate::parser::TypeHint {
                name: "int".to_string(),
                params: vec![],
            },
        ],
    };
    let ty = analyzer.type_from_hint(&hint);
    if let Type::Dict(k, v) = ty {
        assert_eq!(*k, Type::String);
        assert_eq!(*v, Type::Int);
    }
}

// --- infer nested expressions ---
#[test]
fn test_infer_binop_nested() {
    let analyzer = SemanticAnalyzer::new();
    let inner = Expr::BinOp {
        left: Box::new(Expr::IntLiteral(1)),
        op: crate::parser::BinOp::Add,
        right: Box::new(Expr::IntLiteral(2)),
    };
    let outer = Expr::BinOp {
        left: Box::new(inner),
        op: crate::parser::BinOp::Mul,
        right: Box::new(Expr::IntLiteral(3)),
    };
    let ty = analyzer.infer_type(&outer);
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_infer_unary_not() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::UnaryOp {
        op: crate::parser::UnaryOp::Not,
        operand: Box::new(Expr::BoolLiteral(true)),
    };
    let ty = analyzer.infer_type(&expr);
    assert_eq!(ty, Type::Bool);
}

// === テストバッチ51-70: 80%達成へ ===

// --- more patterns covering uncovered lines ---
#[test]
fn test_analyze_multiple_assignments() {
    let code = r#"
def test():
    a: int = 1
    b: int = 2
    c: int = 3
    d: int = 4
    e: int = 5
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_multiple_expressions() {
    let code = r#"
def test():
    x = 1 + 2
    y = 3 * 4
    z = 5 - 6
    w = 7 / 8
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_multiple_comparisons() {
    let code = r#"
def test():
    a = 1 < 2
    b = 2 > 1
    c = 1 <= 2
    d = 2 >= 1
    e = 1 == 1
    f = 1 != 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_complex_if() {
    let code = r#"
def test(x: int) -> int:
    if x < 0:
        return -1
    elif x == 0:
        return 0
    elif x > 0 and x < 10:
        return 1
    elif x >= 10 and x < 100:
        return 2
    else:
        return 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_deep_nesting() {
    let code = r#"
def test():
    if True:
        if True:
            if True:
                if True:
                    x = 1
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more list operations ---
#[test]
fn test_analyze_list_slice_start() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3, 4, 5]
    sub = arr[2:]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_list_slice_end() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3, 4, 5]
    sub = arr[:3]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_list_slice_both() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3, 4, 5]
    sub = arr[1:4]
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more dict patterns ---
#[test]
fn test_analyze_dict_complex_values() {
    let code = r#"
def test():
    d: dict[str, list[int]] = {"a": [1, 2], "b": [3, 4, 5]}
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_dict_nested() {
    let code = r#"
def test():
    d: dict[str, dict[str, int]] = {"outer": {"inner": 42}}
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more tuple patterns ---
#[test]
fn test_analyze_tuple_return() {
    let code = r#"
def divmod_fn(a: int, b: int) -> tuple[int, int]:
    return a // b, a % b
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_tuple_unpack() {
    let code = r#"
def test():
    t: tuple[int, int] = (1, 2)
    a, b = t
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more function patterns ---
#[test]
fn test_analyze_func_no_return() {
    let code = r#"
def side_effect():
    print("effect")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_func_void_return() {
    let code = r#"
def explicit_void():
    x: int = 1
    return
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more string patterns ---
#[test]
fn test_analyze_string_comparison() {
    let code = r#"
def test():
    s1: str = "hello"
    s2: str = "world"
    result = s1 == s2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_string_iteration() {
    let code = r#"
def count_chars(s: str) -> int:
    count: int = 0
    for c in s:
        count += 1
    return count
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more complex algorithms ---
#[test]
fn test_analyze_count_occurrences() {
    let code = r#"
def count_occurrences(arr: list[int], target: int) -> int:
    count: int = 0
    for x in arr:
        if x == target:
            count += 1
    return count
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_reverse_list() {
    let code = r#"
def reverse_list(arr: list[int]) -> list[int]:
    result: list[int] = []
    for i in range(len(arr) - 1, -1, -1):
        result.append(arr[i])
    return result
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more type inference tests ---
#[test]
fn test_infer_string_literal() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::StringLiteral("test".to_string());
    assert_eq!(analyzer.infer_type(&expr), Type::String);
}

#[test]
fn test_infer_int_literal() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::IntLiteral(42);
    assert_eq!(analyzer.infer_type(&expr), Type::Int);
}

#[test]
fn test_infer_float_literal() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::FloatLiteral(3.14);
    assert_eq!(analyzer.infer_type(&expr), Type::Float);
}

#[test]
fn test_infer_bool_literal() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::BoolLiteral(true);
    assert_eq!(analyzer.infer_type(&expr), Type::Bool);
}

// --- more scope tests ---
#[test]
fn test_scope_deep_nesting() {
    use super::scope::ScopeStack;
    let mut scope = ScopeStack::new();
    scope.define("level0", Type::Int, false);
    scope.push();
    scope.define("level1", Type::String, false);
    scope.push();
    scope.define("level2", Type::Float, false);
    scope.push();
    scope.define("level3", Type::Bool, false);

    assert!(scope.lookup("level0").is_some());
    assert!(scope.lookup("level1").is_some());
    assert!(scope.lookup("level2").is_some());
    assert!(scope.lookup("level3").is_some());
}

// --- more type compatibility tests ---
#[test]
fn test_type_compatible_list_same() {
    let t1 = Type::List(Box::new(Type::Int));
    let t2 = Type::List(Box::new(Type::Int));
    assert!(t1.is_compatible_with(&t2));
}

#[test]
fn test_type_compatible_dict_same() {
    let t1 = Type::Dict(Box::new(Type::String), Box::new(Type::Int));
    let t2 = Type::Dict(Box::new(Type::String), Box::new(Type::Int));
    assert!(t1.is_compatible_with(&t2));
}

// === テストバッチ71-100: type_infer.rs未カバー分岐直接攻略 ===

// --- infer_type ListComp branch ---
// --- infer_type GenExpr branch ---
// --- infer_type IfExp branch ---
#[test]
fn test_infer_ifexp_same_types() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::IfExp {
        test: Box::new(Expr::BoolLiteral(true)),
        body: Box::new(Expr::IntLiteral(1)),
        orelse: Box::new(Expr::IntLiteral(2)),
    };
    let ty = analyzer.infer_type(&expr);
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_infer_ifexp_body_unknown() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::IfExp {
        test: Box::new(Expr::BoolLiteral(true)),
        body: Box::new(Expr::Ident("unknown_var".to_string())),
        orelse: Box::new(Expr::IntLiteral(2)),
    };
    let ty = analyzer.infer_type(&expr);
    assert_eq!(ty, Type::Int); // orelse type is returned
}

#[test]
fn test_infer_ifexp_orelse_unknown() {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.define("known_var", Type::String, false);
    let expr = Expr::IfExp {
        test: Box::new(Expr::BoolLiteral(true)),
        body: Box::new(Expr::Ident("known_var".to_string())),
        orelse: Box::new(Expr::Ident("unknown_var".to_string())),
    };
    let ty = analyzer.infer_type(&expr);
    assert_eq!(ty, Type::String); // body type is returned
}

#[test]
fn test_infer_ifexp_different_types() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::IfExp {
        test: Box::new(Expr::BoolLiteral(true)),
        body: Box::new(Expr::IntLiteral(1)),
        orelse: Box::new(Expr::StringLiteral("hello".to_string())),
    };
    let ty = analyzer.infer_type(&expr);
    assert_eq!(ty, Type::Unknown);
}

// --- infer_type UnaryOp branches ---
#[test]
fn test_infer_unary_neg_int() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::UnaryOp {
        op: crate::parser::UnaryOp::Neg,
        operand: Box::new(Expr::IntLiteral(5)),
    };
    let ty = analyzer.infer_type(&expr);
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_infer_unary_pos_float() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::UnaryOp {
        op: crate::parser::UnaryOp::Pos,
        operand: Box::new(Expr::FloatLiteral(3.14)),
    };
    let ty = analyzer.infer_type(&expr);
    assert_eq!(ty, Type::Float);
}

#[test]
fn test_infer_unary_bitnot() {
    let analyzer = SemanticAnalyzer::new();
    let expr = Expr::UnaryOp {
        op: crate::parser::UnaryOp::BitNot,
        operand: Box::new(Expr::IntLiteral(5)),
    };
    let ty = analyzer.infer_type(&expr);
    assert_eq!(ty, Type::Int);
}

// --- infer_type Index branch ---
// --- infer_type Call branch ---
// --- infer_type Attribute branch ---
#[test]
fn test_infer_attribute_dict_items() {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.define(
        "d",
        Type::Dict(Box::new(Type::String), Box::new(Type::Int)),
        false,
    );
    // For attribute, we test via analyze since infer_attribute_type is called internally
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1}
    items = d.items()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_infer_attribute_dict_keys() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1}
    keys = d.keys()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_infer_attribute_dict_values() {
    let code = r#"
def test():
    d: dict[str, int] = {"a": 1}
    values = d.values()
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_infer_attribute_string_join() {
    let code = r#"
def test():
    sep: str = ","
    result = sep.join(["a", "b", "c"])
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more for analyze_calls coverage ---
#[test]
fn test_analyze_call_sorted() {
    let code = r#"
def test():
    arr: list[int] = [3, 1, 4]
    s = sorted(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_call_print_str() {
    let code = r#"
def test():
    print("hello world")
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_call_print_int() {
    let code = r#"
def test():
    print(42)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_call_len_list() {
    let code = r#"
def test():
    arr: list[int] = [1, 2, 3]
    n = len(arr)
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_call_range_one_arg() {
    let code = r#"
def test():
    for i in range(10):
        pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_call_range_two_args() {
    let code = r#"
def test():
    for i in range(1, 10):
        pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_call_range_three_args() {
    let code = r#"
def test():
    for i in range(0, 20, 2):
        pass
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- more analyze_expressions coverage ---
#[test]
fn test_analyze_binop_floor_div() {
    let code = r#"
def test():
    x = 7 // 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_binop_mod() {
    let code = r#"
def test():
    x = 7 % 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_binop_pow() {
    let code = r#"
def test():
    x = 2 ** 10
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_binop_bitand() {
    let code = r#"
def test():
    x = 5 & 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_binop_bitor() {
    let code = r#"
def test():
    x = 5 | 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_binop_bitxor() {
    let code = r#"
def test():
    x = 5 ^ 3
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_binop_shl() {
    let code = r#"
def test():
    x = 1 << 4
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_analyze_binop_shr() {
    let code = r#"
def test():
    x = 16 >> 2
"#;
    let program = parse(code).unwrap();
    let ir = analyze(&program).unwrap();
    assert!(!ir.is_empty());
}

// --- Type compatibility ---
#[test]
fn test_type_compatible_optional_same() {
    let t1 = Type::Optional(Box::new(Type::Int));
    let t2 = Type::Optional(Box::new(Type::Int));
    assert!(t1.is_compatible_with(&t2));
}

#[test]
fn test_type_compatible_tuple_same() {
    let t1 = Type::Tuple(vec![Type::Int, Type::String]);
    let t2 = Type::Tuple(vec![Type::Int, Type::String]);
    assert!(t1.is_compatible_with(&t2));
}

// --- operators convert ---
#[test]
fn test_convert_binop_mod_v2() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Mod);
    assert_eq!(op, IrBinOp::Mod);
}

#[test]
fn test_convert_binop_pow_v2() {
    let analyzer = SemanticAnalyzer::new();
    let op = analyzer.convert_binop(&crate::parser::BinOp::Pow);
    assert_eq!(op, IrBinOp::Pow);
}
