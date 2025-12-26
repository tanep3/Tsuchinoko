//! IR node definitions

use crate::semantic::Type;

/// IR node types
#[derive(Debug, Clone)]
pub enum IrNode {
    /// Variable declaration
    VarDecl {
        name: String,
        ty: Type,
        mutable: bool,
        init: Option<Box<IrExpr>>,
    },
    /// Assignment
    Assign {
        target: String,
        value: Box<IrExpr>,
    },
    /// Function declaration
    FuncDecl {
        name: String,
        params: Vec<(String, Type)>,
        ret: Type,
        body: Vec<IrNode>,
    },
    /// If statement
    If {
        cond: Box<IrExpr>,
        then_block: Vec<IrNode>,
        else_block: Option<Vec<IrNode>>,
    },
    /// For loop
    For {
        var: String,
        var_type: Type,
        iter: Box<IrExpr>,
        body: Vec<IrNode>,
    },
    /// While loop
    While {
        cond: Box<IrExpr>,
        body: Vec<IrNode>,
    },
    /// Return
    Return(Option<Box<IrExpr>>),
    /// Expression statement
    Expr(IrExpr),
}

/// IR expression types
#[derive(Debug, Clone)]
pub enum IrExpr {
    /// Literal values
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    /// Variable reference
    Var(String),
    /// Binary operation
    BinOp {
        left: Box<IrExpr>,
        op: IrBinOp,
        right: Box<IrExpr>,
    },
    /// Unary operation
    UnaryOp {
        op: IrUnaryOp,
        operand: Box<IrExpr>,
    },
    /// Function call
    Call {
        func: String,
        args: Vec<IrExpr>,
    },
    /// List/Vec literal
    List {
        elem_type: Type,
        elements: Vec<IrExpr>,
    },
    /// Range (for loops)
    Range {
        start: Box<IrExpr>,
        end: Box<IrExpr>,
    },
}

/// IR binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum IrBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

/// IR unary operators
#[derive(Debug, Clone)]
pub enum IrUnaryOp {
    Neg,
    Not,
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
        // Just test that we can create it
        if let IrNode::VarDecl { name, .. } = node {
            assert_eq!(name, "x");
        }
    }
}
