//! IR Node (Statement) Definitions
//!
//! 中間表現でのステートメントを定義する。
//! 変数宣言、代入、制御構造、関数定義などを含む。

use crate::semantic::Type;
use super::exprs::IrExpr;
use super::ops::IrAugAssignOp;

/// IR ノード型 (ステートメント)
#[derive(Debug, Clone)]
pub enum IrNode {
    // --- 変数 ---
    /// 変数宣言
    VarDecl {
        name: String,
        ty: Type,
        mutable: bool,
        init: Option<Box<IrExpr>>,
    },
    /// 代入
    Assign { target: String, value: Box<IrExpr> },
    /// インデックス代入 (arr[i] = val)
    IndexAssign {
        target: Box<IrExpr>,
        index: Box<IrExpr>,
        value: Box<IrExpr>,
    },
    /// 累算代入 (x += 1, etc.)
    AugAssign {
        target: String,
        op: IrAugAssignOp,
        value: Box<IrExpr>,
    },
    /// 複数代入 (a, b = val) - タプルアンパック用
    MultiAssign { targets: Vec<String>, value: Box<IrExpr> },
    /// 複数変数宣言 (let (a, b) = val)
    MultiVarDecl {
        targets: Vec<(String, Type, bool)>, // (name, type, mutable)
        value: Box<IrExpr>,
    },
    /// フィールド代入 (self.field = value)
    FieldAssign {
        target: Box<IrExpr>,
        field: String,
        value: Box<IrExpr>,
    },

    // --- 関数・メソッド ---
    /// 関数宣言
    FuncDecl {
        name: String,
        params: Vec<(String, Type)>,
        ret: Type,
        body: Vec<IrNode>,
    },
    /// メソッド宣言 (implブロック内)
    MethodDecl {
        name: String,
        params: Vec<(String, Type)>, // &selfを除く
        ret: Type,
        body: Vec<IrNode>,
        takes_self: bool,     // インスタンスメソッド
        takes_mut_self: bool, // selfを変更する場合
    },

    // --- 制御構造 ---
    /// if文
    If {
        cond: Box<IrExpr>,
        then_block: Vec<IrNode>,
        else_block: Option<Vec<IrNode>>,
    },
    /// forループ
    For {
        var: String,
        var_type: Type,
        iter: Box<IrExpr>,
        body: Vec<IrNode>,
    },
    /// whileループ
    While { cond: Box<IrExpr>, body: Vec<IrNode> },
    /// return文
    Return(Option<Box<IrExpr>>),
    /// break文
    Break,
    /// continue文
    Continue,

    // --- 構造体・型 ---
    /// struct定義 (@dataclass由来)
    StructDef { name: String, fields: Vec<(String, Type)> },
    /// implブロック
    ImplBlock {
        struct_name: String,
        methods: Vec<IrNode>,
    },
    /// 型エイリアス (type Alias = T)
    TypeAlias { name: String, ty: Type },

    // --- 例外・アサート ---
    /// try-exceptブロック (match Resultへ変換)
    TryBlock {
        try_body: Vec<IrNode>,
        except_body: Vec<IrNode>,
    },
    /// アサート文 (V1.3.0)
    Assert {
        test: Box<IrExpr>,
        msg: Option<Box<IrExpr>>,
    },
    /// パニック (raise由来)
    Panic(String),

    // --- その他 ---
    /// 式文
    Expr(IrExpr),
    /// シーケンス (複数トップレベル項目)
    Sequence(Vec<IrNode>),
    /// PyO3 import (numpy, pandas等)
    PyO3Import {
        module: String,
        alias: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_var_decl() {
        let node = IrNode::VarDecl {
            name: "x".to_string(),
            ty: Type::Int,
            mutable: false,
            init: Some(Box::new(IrExpr::IntLit(42))),
        };
        if let IrNode::VarDecl { name, .. } = node {
            assert_eq!(name, "x");
        }
    }
}
