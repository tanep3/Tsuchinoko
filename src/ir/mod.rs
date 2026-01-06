//! IR (Intermediate Representation) module
//!
//! 中間表現の定義を提供する。
//!
//! ## サブモジュール
//! - `ops` - 演算子定義 (IrBinOp, IrUnaryOp, IrAugAssignOp)
//! - `exprs` - 式定義 (IrExpr)
//! - `nodes` - ステートメント定義 (IrNode)
//! - `location` - ソースコード位置情報 (SourceLocation)

pub mod exprs;
pub mod location;
pub mod nodes;
pub mod ops;

// 演算子をre-export
pub use ops::*;
// 式をre-export
pub use exprs::*;
// ノードをre-export
pub use nodes::*;
// 位置情報をre-export
pub use location::*;
