//! Type coercion module
//!
//! 型変換・強制に関するロジックを提供する。
//! - 引数の型強制 (Auto-Ref, Auto-Deref, Auto-Box, Fallback Clone)
//! - 型変換判定

use crate::ir::{IrExpr, IrUnaryOp};
use crate::parser::Expr;

use super::Type;

/// 型変換結果を表す構造体
#[derive(Debug, Clone)]
pub struct CoercionResult {
    /// 変換後の式
    pub expr: IrExpr,
    /// 変換が適用されたかどうか
    pub was_coerced: bool,
}

/// 期待される型から参照情報を展開する
///
/// # Arguments
/// * `expected_ty` - 期待される型
///
/// # Returns
/// (内部型, 参照が必要か, 可変参照が必要か) のタプル
pub fn unpack_expected_type(expected_ty: &Type) -> (Type, bool, bool) {
    match expected_ty {
        Type::MutRef(inner) => (inner.as_ref().clone(), false, true),
        Type::Ref(inner) => (inner.as_ref().clone(), true, false),
        _ => (expected_ty.clone(), false, false),
    }
}

/// 参照を剥がして内部型を取得する
///
/// # Arguments
/// * `ty` - 型
///
/// # Returns
/// 参照を全て剥がした内部型
pub fn strip_references(ty: &Type) -> Type {
    let mut current = ty.clone();
    while let Type::Ref(inner) = current {
        current = *inner;
    }
    current
}

/// Optional型に対してSome()でラップが必要かどうかを判定
///
/// # Arguments
/// * `expected_ty` - 期待される型
/// * `actual_ty` - 実際の型
/// * `expr` - 元の式
///
/// # Returns
/// Some()ラップが必要な場合true
pub fn needs_some_wrap(expected_ty: &Type, actual_ty: &Type, expr: &Expr) -> bool {
    if let Type::Optional(_) = expected_ty {
        // 実際の型がOptionalでなく、Noneリテラルでもない
        !matches!(actual_ty, Type::Optional(_)) && !matches!(expr, Expr::NoneLiteral)
    } else {
        false
    }
}

/// 関数型のBox化が必要かどうかを判定
///
/// # Arguments
/// * `expected_ty` - 期待される型
/// * `actual_ty` - 実際の型
///
/// # Returns
/// Box化が必要な場合true
pub fn needs_box_wrap(expected_ty: &Type, actual_ty: &Type) -> bool {
    matches!(expected_ty, Type::Func { is_boxed: true, .. })
        && matches!(
            actual_ty,
            Type::Func {
                is_boxed: false,
                may_raise: false,
                ..
            }
        )
}

/// 参照の追加が必要かどうかを判定
///
/// # Arguments
/// * `needs_ref` - 参照が期待されているか
/// * `actual_ty` - 実際の型
///
/// # Returns
/// 参照追加が必要な場合true
pub fn needs_reference_wrap(needs_ref: bool, actual_ty: &Type) -> bool {
    needs_ref && !matches!(actual_ty, Type::Ref(_))
}

/// 自動デリファレンスを適用
///
/// Copy型の参照を自動的にデリファレンスする。
///
/// # Arguments
/// * `ir_arg` - 元のIR式
/// * `actual_ty` - 実際の型
///
/// # Returns
/// デリファレンス適用後のIR式と型
pub fn apply_auto_deref(mut ir_arg: IrExpr, actual_ty: &Type) -> (IrExpr, Type) {
    let mut current_ty = actual_ty.clone();
    while let Type::Ref(inner) = &current_ty {
        let inner_ty = inner.as_ref();
        if inner_ty.is_copy() {
            ir_arg = IrExpr::UnaryOp {
                op: IrUnaryOp::Deref,
                operand: Box::new(ir_arg),
            };
            current_ty = inner_ty.clone();
        } else {
            break;
        }
    }
    (ir_arg, current_ty)
}

/// Clone/to_string が必要かどうかを判定
///
/// # Arguments
/// * `ir_arg` - IR式
/// * `resolved_actual` - 解決済みの実際の型
/// * `actual_ty` - 元の実際の型
///
/// # Returns
/// Clone/to_stringが必要な場合true、およびメソッド名
pub fn needs_clone_or_to_string(
    ir_arg: &IrExpr,
    resolved_actual: &Type,
    actual_ty: &Type,
) -> Option<&'static str> {
    // Copy型のメソッド呼び出しはスキップ
    let is_copy_method = matches!(ir_arg, IrExpr::MethodCall { method, .. } if method == "len");

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
        Some(method)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpack_expected_type_ref() {
        let ty = Type::Ref(Box::new(Type::Int));
        let (inner, needs_ref, needs_mut) = unpack_expected_type(&ty);
        assert_eq!(inner, Type::Int);
        assert!(needs_ref);
        assert!(!needs_mut);
    }

    #[test]
    fn test_unpack_expected_type_mut_ref() {
        let ty = Type::MutRef(Box::new(Type::String));
        let (inner, needs_ref, needs_mut) = unpack_expected_type(&ty);
        assert_eq!(inner, Type::String);
        assert!(!needs_ref);
        assert!(needs_mut);
    }

    #[test]
    fn test_unpack_expected_type_normal() {
        let ty = Type::Int;
        let (inner, needs_ref, needs_mut) = unpack_expected_type(&ty);
        assert_eq!(inner, Type::Int);
        assert!(!needs_ref);
        assert!(!needs_mut);
    }

    #[test]
    fn test_strip_references() {
        let ty = Type::Ref(Box::new(Type::Ref(Box::new(Type::Int))));
        assert_eq!(strip_references(&ty), Type::Int);
    }

    #[test]
    fn test_strip_references_no_ref() {
        let ty = Type::String;
        assert_eq!(strip_references(&ty), Type::String);
    }

    #[test]
    fn test_needs_some_wrap_true() {
        let expected = Type::Optional(Box::new(Type::Int));
        let actual = Type::Int;
        let expr = Expr::IntLiteral(42);
        assert!(needs_some_wrap(&expected, &actual, &expr));
    }

    #[test]
    fn test_needs_some_wrap_false_already_optional() {
        let expected = Type::Optional(Box::new(Type::Int));
        let actual = Type::Optional(Box::new(Type::Int));
        let expr = Expr::IntLiteral(42);
        assert!(!needs_some_wrap(&expected, &actual, &expr));
    }

    #[test]
    fn test_needs_some_wrap_false_none_literal() {
        let expected = Type::Optional(Box::new(Type::Int));
        let actual = Type::Unknown;
        let expr = Expr::NoneLiteral;
        assert!(!needs_some_wrap(&expected, &actual, &expr));
    }

    #[test]
    fn test_needs_box_wrap() {
        let expected = Type::Func {
            params: vec![Type::Int],
            ret: Box::new(Type::Int),
            is_boxed: true,
            may_raise: false,
        };
        let actual = Type::Func {
            params: vec![Type::Int],
            ret: Box::new(Type::Int),
            is_boxed: false,
            may_raise: false,
        };
        assert!(needs_box_wrap(&expected, &actual));
    }

    #[test]
    fn test_needs_reference_wrap() {
        assert!(needs_reference_wrap(true, &Type::Int));
        assert!(!needs_reference_wrap(true, &Type::Ref(Box::new(Type::Int))));
        assert!(!needs_reference_wrap(false, &Type::Int));
    }
}
