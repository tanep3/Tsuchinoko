//! AST definitions

/// Expression types
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal
    IntLiteral(i64),
    /// Float literal
    FloatLiteral(f64),
    /// String literal
    StringLiteral(String),
    /// Boolean literal
    BoolLiteral(bool),
    /// None literal
    NoneLiteral,
    /// Identifier
    Ident(String),
    /// Binary operation
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    /// Unary operation
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// Function call
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    /// List literal
    List(Vec<Expr>),
    /// List comprehension [elt for target in iter]
    ListComp {
        elt: Box<Expr>,
        target: String,
        iter: Box<Expr>,
    },
    /// Tuple literal
    Tuple(Vec<Expr>),
    /// Index access
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    /// Attribute access (obj.attr)
    Attribute {
        value: Box<Expr>,
        attr: String,
    },
    /// Dict literal
    Dict(Vec<(Expr, Expr)>),
    /// f-string literal f"..."
    FString {
        /// Static parts of the f-string
        parts: Vec<String>,
        /// Expressions to interpolate
        values: Vec<Expr>,
    },
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    In,  // x in dict
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
}

/// Statement types
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Variable assignment
    Assign {
        target: String,
        type_hint: Option<TypeHint>,
        value: Expr,
    },
    /// Index assignment (arr[i] = val)
    IndexAssign {
        target: Expr,
        index: Expr,
        value: Expr,
    },
    /// Tuple unpacking assignment (a, b = func())
    TupleAssign {
        targets: Vec<String>,
        value: Expr,
    },
    /// Function definition
    FuncDef {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeHint>,
        body: Vec<Stmt>,
    },
    /// If statement
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        elif_clauses: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
    },
    /// For loop
    For {
        target: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    /// While loop
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    /// Return statement
    Return(Option<Expr>),
    /// Expression statement
    Expr(Expr),
    /// Class definition (dataclass -> struct)
    ClassDef {
        name: String,
        fields: Vec<Field>,
    },
    /// Try-except statement
    TryExcept {
        try_body: Vec<Stmt>,
        except_type: Option<String>,
        except_body: Vec<Stmt>,
    },
}

/// Function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_hint: Option<TypeHint>,
}

/// Type hint
#[derive(Debug, Clone, PartialEq)]
pub struct TypeHint {
    pub name: String,
    pub params: Vec<TypeHint>,
}

/// Class field (for dataclass -> struct)
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub type_hint: TypeHint,
}

/// Program (collection of statements)
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_int_literal() {
        let expr = Expr::IntLiteral(42);
        assert_eq!(expr, Expr::IntLiteral(42));
    }

    #[test]
    fn test_stmt_assign() {
        let stmt = Stmt::Assign {
            target: "x".to_string(),
            type_hint: Some(TypeHint {
                name: "int".to_string(),
                params: vec![],
            }),
            value: Expr::IntLiteral(10),
        };
        if let Stmt::Assign { target, type_hint, value } = stmt {
            assert_eq!(target, "x");
            assert!(type_hint.is_some());
            assert_eq!(value, Expr::IntLiteral(10));
        }
    }
}
