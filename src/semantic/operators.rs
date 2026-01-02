//! Operator conversion module
//!
//! 演算子の AST → IR 変換処理を提供する。
//! - 二項演算子 (BinOp) の変換
//! - 累算代入演算子 (AugAssign) の変換

use crate::ir::{IrAugAssignOp, IrBinOp};
use crate::parser::{AugAssignOp, BinOp as AstBinOp};

/// AST の BinOp を IR の IrBinOp に変換する
///
/// # Arguments
/// * `op` - パーサーが生成した AstBinOp
///
/// # Returns
/// 対応する IrBinOp
pub fn convert_binop(op: &AstBinOp) -> IrBinOp {
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
        AstBinOp::NotIn => IrBinOp::NotContains,
        AstBinOp::Is => IrBinOp::Is,
        AstBinOp::IsNot => IrBinOp::IsNot,
        // Bitwise operators
        AstBinOp::BitAnd => IrBinOp::BitAnd,
        AstBinOp::BitOr => IrBinOp::BitOr,
        AstBinOp::BitXor => IrBinOp::BitXor,
        AstBinOp::Shl => IrBinOp::Shl,
        AstBinOp::Shr => IrBinOp::Shr,
        AstBinOp::MatMul => IrBinOp::MatMul,
    }
}

/// AST の AugAssignOp を IR の IrAugAssignOp に変換する
///
/// # Arguments
/// * `op` - パーサーが生成した AugAssignOp
///
/// # Returns
/// 対応する IrAugAssignOp
pub fn convert_aug_assign_op(op: &AugAssignOp) -> IrAugAssignOp {
    match op {
        AugAssignOp::Add => IrAugAssignOp::Add,
        AugAssignOp::Sub => IrAugAssignOp::Sub,
        AugAssignOp::Mul => IrAugAssignOp::Mul,
        AugAssignOp::Div => IrAugAssignOp::Div,
        AugAssignOp::Mod => IrAugAssignOp::Mod,
        AugAssignOp::FloorDiv => IrAugAssignOp::FloorDiv,
        AugAssignOp::Pow => IrAugAssignOp::Pow,
        // Bitwise operators
        AugAssignOp::BitAnd => IrAugAssignOp::BitAnd,
        AugAssignOp::BitOr => IrAugAssignOp::BitOr,
        AugAssignOp::BitXor => IrAugAssignOp::BitXor,
        AugAssignOp::Shl => IrAugAssignOp::Shl,
        AugAssignOp::Shr => IrAugAssignOp::Shr,
    }
}

/// 演算子が比較演算子かどうかを判定
///
/// # Arguments
/// * `op` - 判定対象の AstBinOp
///
/// # Returns
/// 比較演算子の場合 true
pub fn is_comparison_op(op: &AstBinOp) -> bool {
    matches!(
        op,
        AstBinOp::Eq
            | AstBinOp::NotEq
            | AstBinOp::Lt
            | AstBinOp::Gt
            | AstBinOp::LtEq
            | AstBinOp::GtEq
            | AstBinOp::In
            | AstBinOp::NotIn
            | AstBinOp::Is
            | AstBinOp::IsNot
    )
}

/// 演算子が論理演算子かどうかを判定
///
/// # Arguments
/// * `op` - 判定対象の AstBinOp
///
/// # Returns
/// 論理演算子の場合 true
pub fn is_logical_op(op: &AstBinOp) -> bool {
    matches!(op, AstBinOp::And | AstBinOp::Or)
}

/// 演算子がビット演算子かどうかを判定
///
/// # Arguments
/// * `op` - 判定対象の AstBinOp
///
/// # Returns
/// ビット演算子の場合 true
pub fn is_bitwise_op(op: &AstBinOp) -> bool {
    matches!(
        op,
        AstBinOp::BitAnd | AstBinOp::BitOr | AstBinOp::BitXor | AstBinOp::Shl | AstBinOp::Shr
    )
}

/// 演算子が算術演算子かどうかを判定
///
/// # Arguments
/// * `op` - 判定対象の AstBinOp
///
/// # Returns
/// 算術演算子の場合 true
pub fn is_arithmetic_op(op: &AstBinOp) -> bool {
    matches!(
        op,
        AstBinOp::Add
            | AstBinOp::Sub
            | AstBinOp::Mul
            | AstBinOp::Div
            | AstBinOp::FloorDiv
            | AstBinOp::Mod
            | AstBinOp::Pow
            | AstBinOp::MatMul
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_binop_arithmetic() {
        assert_eq!(convert_binop(&AstBinOp::Add), IrBinOp::Add);
        assert_eq!(convert_binop(&AstBinOp::Sub), IrBinOp::Sub);
        assert_eq!(convert_binop(&AstBinOp::Mul), IrBinOp::Mul);
        assert_eq!(convert_binop(&AstBinOp::Div), IrBinOp::Div);
    }

    #[test]
    fn test_convert_binop_comparison() {
        assert_eq!(convert_binop(&AstBinOp::Eq), IrBinOp::Eq);
        assert_eq!(convert_binop(&AstBinOp::Lt), IrBinOp::Lt);
        assert_eq!(convert_binop(&AstBinOp::Gt), IrBinOp::Gt);
    }

    #[test]
    fn test_convert_binop_bitwise() {
        assert_eq!(convert_binop(&AstBinOp::BitAnd), IrBinOp::BitAnd);
        assert_eq!(convert_binop(&AstBinOp::BitOr), IrBinOp::BitOr);
        assert_eq!(convert_binop(&AstBinOp::Shl), IrBinOp::Shl);
    }

    #[test]
    fn test_convert_binop_contains() {
        assert_eq!(convert_binop(&AstBinOp::In), IrBinOp::Contains);
        assert_eq!(convert_binop(&AstBinOp::NotIn), IrBinOp::NotContains);
    }

    #[test]
    fn test_convert_aug_assign() {
        assert_eq!(convert_aug_assign_op(&AugAssignOp::Add), IrAugAssignOp::Add);
        assert_eq!(convert_aug_assign_op(&AugAssignOp::Sub), IrAugAssignOp::Sub);
        assert_eq!(convert_aug_assign_op(&AugAssignOp::Pow), IrAugAssignOp::Pow);
    }

    #[test]
    fn test_is_comparison_op() {
        assert!(is_comparison_op(&AstBinOp::Eq));
        assert!(is_comparison_op(&AstBinOp::Lt));
        assert!(is_comparison_op(&AstBinOp::In));
        assert!(!is_comparison_op(&AstBinOp::Add));
    }

    #[test]
    fn test_is_logical_op() {
        assert!(is_logical_op(&AstBinOp::And));
        assert!(is_logical_op(&AstBinOp::Or));
        assert!(!is_logical_op(&AstBinOp::Eq));
    }

    #[test]
    fn test_is_bitwise_op() {
        assert!(is_bitwise_op(&AstBinOp::BitAnd));
        assert!(is_bitwise_op(&AstBinOp::Shl));
        assert!(!is_bitwise_op(&AstBinOp::Add));
    }

    #[test]
    fn test_is_arithmetic_op() {
        assert!(is_arithmetic_op(&AstBinOp::Add));
        assert!(is_arithmetic_op(&AstBinOp::MatMul));
        assert!(!is_arithmetic_op(&AstBinOp::Eq));
    }
}
