//! 組み込み関数の宣言的定義テーブル
//!
//! このファイルでは、Python の組み込み関数がどのように Rust へ変換されるか、
//! およびどのような型を返すかを宣言的に定義する。

use crate::ir::exprs::{BuiltinId, IntrinsicOp};
use crate::semantic::Type;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// 組み込み関数の展開形式
pub enum BuiltinKind {
    /// ブリッジ経由 (BridgeCall に展開)
    /// Python 側で実行され、結果が TnkValue として返る。
    Bridge { target: &'static str },
    /// Rust ネイティブメソッド (第一引数がレシーバ: args[0].method(args[1..]))
    /// 高速な実行が可能。
    NativeMethod { method: &'static str },
    /// 言語組み込みの特殊演算 (論理演算、特殊な制御フロー等)
    Intrinsic { op: IntrinsicOp },
}

/// 組み込み関数の仕様定義
pub struct BuiltinSpec {
    pub id: BuiltinId,
    /// 解決済みの正規化名 (例: "len", "pd.read_csv")
    pub name: &'static str,
    pub kind: BuiltinKind,
    /// 戻り値の型を解決する純粋関数
    pub ret_ty_resolver: fn(args: &[Type]) -> Type,
}

/// 組み込み関数の登録リスト
pub const BUILTIN_SPECS: &[BuiltinSpec] = &[
    BuiltinSpec {
        id: BuiltinId::Len,
        name: "len",
        kind: BuiltinKind::NativeMethod { method: "len" },
        ret_ty_resolver: |_| Type::Int,
    },
    BuiltinSpec {
        id: BuiltinId::Sum,
        name: "sum",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            if args.is_empty() {
                return Type::Int;
            }
            match &args[0] {
                Type::List(inner) | Type::Set(inner) => (**inner).clone(),
                _ => Type::Int,
            }
        },
    },
    BuiltinSpec {
        id: BuiltinId::Any,
        name: "any",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Bool,
    },
    BuiltinSpec {
        id: BuiltinId::All,
        name: "all",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Bool,
    },
    BuiltinSpec {
        id: BuiltinId::Abs,
        name: "abs",
        kind: BuiltinKind::NativeMethod { method: "abs" },
        ret_ty_resolver: |args| {
            if args.is_empty() {
                return Type::Int;
            }
            args[0].clone()
        },
    },
    BuiltinSpec {
        id: BuiltinId::Min,
        name: "min",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            if args.is_empty() {
                return Type::Unknown;
            }
            args[0].clone()
        },
    },
    BuiltinSpec {
        id: BuiltinId::Max,
        name: "max",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            if args.is_empty() {
                return Type::Unknown;
            }
            args[0].clone()
        },
    },
    BuiltinSpec {
        id: BuiltinId::Round,
        name: "round",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            if args.len() >= 2 {
                Type::Float
            } else {
                Type::Int
            }
        },
    },
    BuiltinSpec {
        id: BuiltinId::Input,
        name: "input",
        kind: BuiltinKind::Bridge { target: "input" },
        ret_ty_resolver: |_| Type::String,
    },
    BuiltinSpec {
        id: BuiltinId::Chr,
        name: "chr",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::String,
    },
    BuiltinSpec {
        id: BuiltinId::Ord,
        name: "ord",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Int,
    },
    BuiltinSpec {
        id: BuiltinId::Bin,
        name: "bin",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::String,
    },
    BuiltinSpec {
        id: BuiltinId::Hex,
        name: "hex",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::String,
    },
    BuiltinSpec {
        id: BuiltinId::Oct,
        name: "oct",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::String,
    },
    BuiltinSpec {
        id: BuiltinId::Print,
        name: "print",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Unit,
    },
    BuiltinSpec {
        id: BuiltinId::Range,
        name: "range",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Unknown, // Range type not yet defined
    },
    BuiltinSpec {
        id: BuiltinId::Enumerate,
        name: "enumerate",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            if args.is_empty() {
                return Type::Unknown;
            }
            let inner = match &args[0] {
                Type::List(e) | Type::Set(e) => *e.clone(),
                Type::Dict(k, _) => *k.clone(),
                _ => Type::Unknown,
            };
            Type::List(Box::new(Type::Tuple(vec![Type::Int, inner])))
        },
    },
    BuiltinSpec {
        id: BuiltinId::Zip,
        name: "zip",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            let mut elem_types = Vec::new();
            for arg in args {
                let inner = match arg {
                    Type::List(e) | Type::Set(e) => *e.clone(),
                    Type::Dict(k, _) => *k.clone(),
                    _ => Type::Unknown,
                };
                elem_types.push(inner);
            }
            Type::List(Box::new(Type::Tuple(elem_types)))
        },
    },
    BuiltinSpec {
        id: BuiltinId::Int,
        name: "int",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Int,
    },
    BuiltinSpec {
        id: BuiltinId::Float,
        name: "float",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Float,
    },
    BuiltinSpec {
        id: BuiltinId::Str,
        name: "str",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::String,
    },
    BuiltinSpec {
        id: BuiltinId::List,
        name: "list",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            if args.is_empty() {
                return Type::List(Box::new(Type::Unknown));
            }
            match &args[0] {
                Type::List(inner) | Type::Set(inner) => Type::List(inner.clone()),
                Type::Dict(k, _) => Type::List(Box::new((**k).clone())),
                _ => Type::List(Box::new(Type::Unknown)),
            }
        },
    },
    BuiltinSpec {
        id: BuiltinId::Tuple,
        name: "tuple",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Any,
    },
    BuiltinSpec {
        id: BuiltinId::Dict,
        name: "dict",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown)),
    },
    BuiltinSpec {
        id: BuiltinId::IsInstance,
        name: "isinstance",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Bool,
    },
    BuiltinSpec {
        id: BuiltinId::Open,
        name: "open",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |_| Type::Unknown, // TODO: File type
    },
    BuiltinSpec {
        id: BuiltinId::Sorted,
        name: "sorted",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            if args.is_empty() {
                return Type::List(Box::new(Type::Unknown));
            }
            match &args[0] {
                Type::List(inner) | Type::Set(inner) => Type::List(inner.clone()),
                Type::Dict(k, _) => Type::List(Box::new((**k).clone())),
                _ => Type::List(Box::new(Type::Unknown)),
            }
        },
    },
    BuiltinSpec {
        id: BuiltinId::Set,
        name: "set",
        kind: BuiltinKind::Intrinsic {
            op: IntrinsicOp::Basic,
        },
        ret_ty_resolver: |args| {
            if args.is_empty() {
                return Type::Set(Box::new(Type::Unknown));
            }
            match &args[0] {
                Type::List(inner) | Type::Set(inner) => Type::Set(inner.clone()),
                Type::Dict(k, _) => Type::Set(Box::new((**k).clone())),
                _ => Type::Set(Box::new(Type::Unknown)),
            }
        },
    },
];

/// 名前から BuiltinSpec を高速に検索するためのマップ
pub static BUILTIN_MAP: Lazy<HashMap<&'static str, &'static BuiltinSpec>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for spec in BUILTIN_SPECS {
        m.insert(spec.name, spec);
    }
    m
});

/// 名前から仕様を取得
pub fn get_builtin_spec(name: &str) -> Option<&'static BuiltinSpec> {
    BUILTIN_MAP.get(name).copied()
}
