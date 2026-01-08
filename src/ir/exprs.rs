//! IR Expression Definitions
//!
//! 中間表現での式を定義する。
//! リテラル、変数参照、演算、関数呼び出し、その他の式を含む。

use super::nodes::IrNode;
use super::ops::{IrBinOp, IrUnaryOp};
use crate::semantic::Type;

/// IR 式の型
#[derive(Debug, Clone)]
pub enum IrExpr {
    // --- リテラル ---
    /// 整数リテラル
    IntLit(i64),
    /// 浮動小数点リテラル
    FloatLit(f64),
    /// 文字列リテラル
    StringLit(String),
    /// 真偽値リテラル
    BoolLit(bool),
    /// Noneリテラル (Rust None)
    NoneLit,

    // --- 変数・フィールド ---
    /// 変数参照
    Var(String),
    /// フィールドアクセス (e.g., obj.field)
    FieldAccess { target: Box<IrExpr>, field: String },

    // --- 演算 ---
    /// 二項演算
    BinOp {
        left: Box<IrExpr>,
        op: IrBinOp,
        right: Box<IrExpr>,
    },
    /// 単項演算
    UnaryOp { op: IrUnaryOp, operand: Box<IrExpr> },

    // --- 呼び出し ---
    /// 関数呼び出し
    Call {
        func: Box<IrExpr>,
        args: Vec<IrExpr>,
        /// V1.5.2: 呼び出し先が may_raise (Result を返す) かどうか
        callee_may_raise: bool,
    },
    /// メソッド呼び出し (e.g., arr.len())
    MethodCall {
        target: Box<IrExpr>,
        method: String,
        args: Vec<IrExpr>,
        /// ターゲットの型 (Set/List/Dict/Class 判別用)
        target_type: Type,
    },
    /// PyO3モジュール呼び出し (np.array(...), pd.DataFrame(...))
    PyO3Call {
        module: String,
        method: String,
        args: Vec<IrExpr>,
    },
    /// PyO3メソッド呼び出し (Type::Any対象)
    PyO3MethodCall {
        target: Box<IrExpr>,
        method: String,
        args: Vec<IrExpr>,
    },

    // --- コレクション ---
    /// リスト/Vec リテラル
    List {
        elem_type: Type,
        elements: Vec<IrExpr>,
    },
    /// タプルリテラル
    Tuple(Vec<IrExpr>),
    /// 辞書/HashMap リテラル
    Dict {
        key_type: Type,
        value_type: Type,
        entries: Vec<(IrExpr, IrExpr)>,
    },
    /// セット/HashSet リテラル (V1.5.0)
    Set {
        elem_type: Type,
        elements: Vec<IrExpr>,
    },

    // --- 内包表記 ---
    /// リスト内包表記 [elt for target in iter if condition]
    ListComp {
        elt: Box<IrExpr>,
        target: String,
        iter: Box<IrExpr>,
        condition: Option<Box<IrExpr>>,
    },
    /// セット内包表記 {elt for target in iter if condition} (V1.6.0)
    SetComp {
        elt: Box<IrExpr>,
        target: String,
        iter: Box<IrExpr>,
        condition: Option<Box<IrExpr>>,
    },
    /// 辞書内包表記 {k: v for target in iter if condition} (V1.3.0)
    DictComp {
        key: Box<IrExpr>,
        value: Box<IrExpr>,
        target: String,
        iter: Box<IrExpr>,
        condition: Option<Box<IrExpr>>,
    },

    // --- アクセス ---
    /// インデックスアクセス
    Index {
        target: Box<IrExpr>,
        index: Box<IrExpr>,
    },
    /// スライスアクセス (target[start..end:step])
    Slice {
        target: Box<IrExpr>,
        start: Option<Box<IrExpr>>,
        end: Option<Box<IrExpr>>,
        step: Option<Box<IrExpr>>, // V1.5.0: step for arr[::2], arr[::-1]
    },
    /// Range (forループ用)
    Range {
        start: Box<IrExpr>,
        end: Box<IrExpr>,
    },

    // --- 参照 ---
    /// 参照 (&expr)
    Reference { target: Box<IrExpr> },
    /// 可変参照 (&mut expr)
    MutReference { target: Box<IrExpr> },

    // --- 特殊 ---
    /// print文 (型情報付きフォーマット選択)
    Print { args: Vec<(IrExpr, Type)> },
    /// クロージャ (lambda / ネスト関数)
    Closure {
        params: Vec<String>,
        body: Vec<IrNode>,
        ret_type: Type,
    },
    /// f-string (format! マクロ)
    FString {
        parts: Vec<String>,
        values: Vec<IrExpr>,
    },
    /// 条件式 (if test { body } else { orelse })
    IfExp {
        test: Box<IrExpr>,
        body: Box<IrExpr>,
        orelse: Box<IrExpr>,
    },
    /// Optionアンラップ (.unwrap()生成)
    Unwrap(Box<IrExpr>),
    /// Box::new ヘルパー
    BoxNew(Box<IrExpr>),
    /// 明示的キャスト (expr as type)
    Cast { target: Box<IrExpr>, ty: String },
    /// 生Rustコード (IRで表現できないパターン用)
    RawCode(String),
    /// JSON変換 (PyO3戻り値用)
    JsonConversion {
        target: Box<IrExpr>,
        convert_to: String,
    },
    /// V1.3.1: struct構築 (semantic側で判定)
    StructConstruct {
        name: String,
        fields: Vec<(String, IrExpr)>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_expr_int_lit() {
        let expr = IrExpr::IntLit(42);
        if let IrExpr::IntLit(n) = expr {
            assert_eq!(n, 42);
        }
    }

    #[test]
    fn test_ir_expr_var() {
        let expr = IrExpr::Var("x".to_string());
        if let IrExpr::Var(name) = expr {
            assert_eq!(name, "x");
        }
    }
}
