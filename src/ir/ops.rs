//! IR Operator Definitions
use serde::{Deserialize, Serialize};
//
// 中間表現での演算子を定義する。
// - 二項演算子 (IrBinOp)
// - 単項演算子 (IrUnaryOp)
// - 累算代入演算子 (IrAugAssignOp)

/// IR 二項演算子
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IrBinOp {
    // 算術演算子
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,

    // 比較演算子
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // 論理演算子
    And,
    Or,

    // 包含演算子
    Contains,    // x in dict -> dict.contains_key(&x)
    NotContains, // x not in dict -> !dict.contains_key(&x) (V1.3.0)

    // 同一性演算子
    Is,    // x is None -> x.is_none()
    IsNot, // x is not None -> x.is_some()

    // ビット演算子 (V1.3.0)
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    Shl,    // <<
    Shr,    // >>

    // 行列乗算 (V1.3.0)
    MatMul, // @
}

/// IR 単項演算子
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IrUnaryOp {
    /// 符号反転
    Neg,
    /// 論理否定
    Not,
    /// デリファレンス (*expr)
    Deref,
    /// ビット否定 (V1.3.0)
    BitNot,
}

/// IR 累算代入演算子
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IrAugAssignOp {
    Add,      // +=
    Sub,      // -=
    Mul,      // *=
    Div,      // /=
    FloorDiv, // //=
    Mod,      // %=
    // V1.3.0 additions
    Pow,    // **=
    BitAnd, // &=
    BitOr,  // |=
    BitXor, // ^=
    Shl,    // <<=
    Shr,    // >>=
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binop_partial_eq() {
        assert_eq!(IrBinOp::Add, IrBinOp::Add);
        assert_ne!(IrBinOp::Add, IrBinOp::Sub);
    }

    #[test]
    fn test_aug_assign_op_partial_eq() {
        assert_eq!(IrAugAssignOp::Add, IrAugAssignOp::Add);
        assert_ne!(IrAugAssignOp::Add, IrAugAssignOp::Mul);
    }
}
