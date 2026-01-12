use super::nodes::IrNode;
use super::ops::{IrBinOp, IrUnaryOp};
use crate::semantic::Type;
use serde::{Deserialize, Serialize};

/// 式の一意な ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExprId(pub u32);

/// 組み込み関数の識別子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinId {
    Len,
    Sum,
    Any,
    All,
    Range,
    Enumerate,
    Zip,
    Abs,
    Min,
    Max,
    Round,
    Input,
    Chr,
    Ord,
    Bin,
    Hex,
    Oct,
    Print,      // V1.7.0
    IsInstance, // V1.7.0
    Open,       // V1.7.0
}

impl BuiltinId {
    /// Rust での対応する関数名を返す
    pub fn to_rust_name(&self) -> &'static str {
        match self {
            BuiltinId::Len => "len",
            BuiltinId::Sum => "sum",
            BuiltinId::Any => "any",
            BuiltinId::All => "all",
            BuiltinId::Range => "range",
            BuiltinId::Enumerate => "enumerate",
            BuiltinId::Zip => "zip",
            BuiltinId::Abs => "abs",
            BuiltinId::Min => "min",
            BuiltinId::Max => "max",
            BuiltinId::Round => "round",
            BuiltinId::Input => "input",
            BuiltinId::Chr => "chr",
            BuiltinId::Ord => "ord",
            BuiltinId::Bin => "bin",
            BuiltinId::Hex => "hex",
            BuiltinId::Oct => "oct",
            BuiltinId::Print => "print",
            BuiltinId::IsInstance => "isinstance",
            BuiltinId::Open => "open",
        }
    }
}

/// 言語組み込みの特殊演算 (論理演算など)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntrinsicOp {
    // 基本的な組み込み関数扱い
    Basic,
}

/// IR 式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrExpr {
    pub id: ExprId,
    pub kind: IrExprKind,
}

/// IR 式の型（実体）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IrExprKind {
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
        /// V1.7.0: 呼び出し先が PythonBridge を必要とするかどうか
        callee_needs_bridge: bool,
    },
    /// メソッド呼び出し (e.g., arr.len())
    MethodCall {
        target: Box<IrExpr>,
        method: String,
        args: Vec<IrExpr>,
        /// ターゲットの型 (Set/List/Dict/Class 判別用)
        target_type: Type,
        /// V1.7.0: 呼び出し先が PythonBridge を必要とするかどうか
        callee_needs_bridge: bool,
    },
    /// PyO3モジュール呼び出し (np.array(...), pd.DataFrame(...))
    PyO3Call {
        module: String,
        method: String,
        args: Vec<IrExpr>,
    },
    /// PyO3メソッド呼び出し (Type::Any対象) - Deprecated in V1.7.0 favor of BridgeMethodCall
    PyO3MethodCall {
        target: Box<IrExpr>,
        method: String,
        args: Vec<IrExpr>,
    },
    /// V1.7.0: ブリッジ経由のメソッド呼び出し (obj.method(...))
    BridgeMethodCall {
        target: Box<IrExpr>,
        method: String,
        args: Vec<IrExpr>,
        keywords: Vec<(String, IrExpr)>,
    },
    /// V1.7.0: ブリッジ経由の直接呼び出し (func(...))
    BridgeCall {
        target: Box<IrExpr>,
        args: Vec<IrExpr>,
        keywords: Vec<(String, IrExpr)>,
    },
    /// V1.7.0: ブリッジ経由の属性アクセス (obj.attr)
    BridgeAttributeAccess {
        target: Box<IrExpr>,
        attribute: String,
    },
    /// V1.7.0: ブリッジ経由のアイテムアクセス (obj[key])
    BridgeItemAccess {
        target: Box<IrExpr>,
        index: Box<IrExpr>,
    },
    /// V1.7.0: ブリッジ経由のスライス (obj[start:stop:step])
    BridgeSlice {
        target: Box<IrExpr>,
        start: Box<IrExpr>,
        stop: Box<IrExpr>,
        step: Box<IrExpr>,
    },
    /// V1.7.0: 構造化された組み込み関数呼び出し
    BuiltinCall {
        id: BuiltinId,
        args: Vec<IrExpr>,
    },

    // --- Conversions (V1.7.0 Option B) ---
    /// Reference (&expr) - Used for Zero-Copy Bridge calls
    Ref(Box<IrExpr>),
    /// TnkValue conversion (TnkValue::from(expr))
    TnkValueFrom(Box<IrExpr>),
    /// V1.7.0: ブリッジ経由のモジュール取得 (bridge.get("alias"))
    BridgeGet { alias: String },
    /// V1.7.0: ブリッジの戻り値 (TnkValue) から期待型への標準変換
    FromTnkValue {
        value: Box<IrExpr>,
        to_type: Type,
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
        values: Vec<(IrExpr, Type)>,
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
    /// V1.6.0: DynamicValue でラップ (isinstance 対応)
    DynamicWrap {
        enum_name: String, // "DynamicValue"
        variant: String,   // "Int", "Str", etc.
        value: Box<IrExpr>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_expr_int_lit() {
        let expr = IrExpr { id: ExprId(0), kind: IrExprKind::IntLit(42) };
        if let IrExprKind::IntLit(n) = expr.kind {
            assert_eq!(n, 42);
        }
    }

    #[test]
    fn test_ir_expr_var() {
        let expr = IrExpr { id: ExprId(1), kind: IrExprKind::Var("x".to_string()) };
        if let IrExprKind::Var(name) = expr.kind {
            assert_eq!(name, "x");
        }
    }
}
