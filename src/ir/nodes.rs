//! IR Node (Statement) Definitions
//!
//! 中間表現でのステートメントを定義する。
//! 変数宣言、代入、制御構造、関数定義などを含む。

use super::exprs::IrExpr;
use super::ops::IrAugAssignOp;
use crate::semantic::Type;

/// ホイストが必要な変数（ブロック境界を越えて使用される）
/// Python の関数スコープを Rust のブロックスコープに変換するために使用
#[derive(Debug, Clone, PartialEq)]
pub struct HoistedVar {
    pub name: String,
    pub ty: Type,
}

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
    MultiAssign {
        targets: Vec<String>,
        value: Box<IrExpr>,
    },
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
        hoisted_vars: Vec<HoistedVar>, // 関数スコープにホイストが必要な変数
        may_raise: bool,               // 例外を発生しうる関数か（Result化が必要）
    },
    /// メソッド宣言 (implブロック内)
    MethodDecl {
        name: String,
        params: Vec<(String, Type)>, // &selfを除く
        ret: Type,
        body: Vec<IrNode>,
        takes_self: bool,     // インスタンスメソッド
        takes_mut_self: bool, // selfを変更する場合
        may_raise: bool,      // V1.5.2: 例外を発生しうるメソッドか
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
    While {
        cond: Box<IrExpr>,
        body: Vec<IrNode>,
    },
    /// return文
    Return(Option<Box<IrExpr>>),
    /// break文
    Break,
    /// continue文
    Continue,

    // --- 構造体・型 ---
    /// struct定義 (@dataclass由来)
    /// V1.6.0: base field for composition (inheritance)
    StructDef {
        name: String,
        fields: Vec<(String, Type)>,
        base: Option<String>,  // V1.6.0: コンポジション用の親クラス名
    },
    /// implブロック
    ImplBlock {
        struct_name: String,
        methods: Vec<IrNode>,
    },
    /// 型エイリアス (type Alias = T)
    TypeAlias { name: String, ty: Type },

    // --- 例外・アサート ---
    /// try-exceptブロック (V1.5.2: except_var, else_body 追加)
    TryBlock {
        try_body: Vec<IrNode>,
        except_body: Vec<IrNode>,
        except_var: Option<String>,     // V1.5.2: except ... as e の変数名
        else_body: Option<Vec<IrNode>>, // V1.5.2: else ブロック
        finally_body: Option<Vec<IrNode>>,
    },
    /// アサート文 (V1.3.0)
    Assert {
        test: Box<IrExpr>,
        msg: Option<Box<IrExpr>>,
    },
    /// Raise 文 (V1.5.2: cause 対応, 行番号対応)
    /// raise ValueError("message") from original_error
    Raise {
        exc_type: String,
        message: Box<IrExpr>,
        cause: Option<Box<IrExpr>>, // V1.5.2: from 句
        line: usize,                // V1.5.2: ソースコード行番号（0 = 不明）
    },

    // --- その他 ---
    /// 式文
    Expr(IrExpr),
    /// シーケンス (複数トップレベル項目)
    Sequence(Vec<IrNode>),
    /// PyO3 import (numpy, pandas等)
    /// V1.4.0: items フィールドを追加し、from import の個別関数名を保持
    PyO3Import {
        module: String,
        alias: Option<String>,
        /// For "from module import a, b, c" - contains ["a", "b", "c"]
        items: Option<Vec<String>>,
    },
    /// V1.6.0: スコープブロック (with 文から生成)
    Block {
        stmts: Vec<IrNode>,
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
