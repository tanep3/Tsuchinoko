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
    UnaryOp { op: UnaryOp, operand: Box<Expr> },
    /// Function call with positional and keyword arguments
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        kwargs: Vec<(String, Expr)>,
    },
    /// List literal
    List(Vec<Expr>),
    /// List comprehension [elt for target in iter] or [elt for target in iter if cond]
    ListComp {
        elt: Box<Expr>,
        target: String,
        iter: Box<Expr>,
        condition: Option<Box<Expr>>,
    },
    /// Dict comprehension {key: value for target in iter} or {k: v for k, v in items if cond} (V1.3.0)
    DictComp {
        key: Box<Expr>,
        value: Box<Expr>,
        target: String,
        iter: Box<Expr>,
        condition: Option<Box<Expr>>,
    },
    /// Generator expression (elt for target in iter if cond) - used in function calls
    GenExpr {
        elt: Box<Expr>,
        target: String,
        iter: Box<Expr>,
        condition: Option<Box<Expr>>,
    },
    /// Conditional Expression (body if test else orelse)
    IfExp {
        test: Box<Expr>,
        body: Box<Expr>,
        orelse: Box<Expr>,
    },
    /// Tuple literal
    Tuple(Vec<Expr>),
    /// Index access
    Index { target: Box<Expr>, index: Box<Expr> },
    /// Slice access (target[start:end])
    Slice {
        target: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
    },
    /// Attribute access (obj.attr)
    Attribute { value: Box<Expr>, attr: String },
    /// Dict literal
    Dict(Vec<(Expr, Expr)>),
    /// f-string literal f"..."
    FString {
        /// Static parts of the f-string
        parts: Vec<String>,
        /// Expressions to interpolate
        values: Vec<Expr>,
    },
    /// Lambda expression (lambda params: body)
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    /// Starred expression (*expr) for unpacking
    Starred(Box<Expr>),
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
    In,    // x in dict
    NotIn, // x not in dict  (V1.3.0)
    Is,    // x is None
    IsNot, // x is not None
    // Bitwise operators (V1.3.0)
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    Shl,    // <<
    Shr,    // >>
    // Matrix multiplication (V1.3.0)
    MatMul, // @
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
    BitNot, // ~ (V1.3.0)
}

/// Augmented assignment operators
#[derive(Debug, Clone, PartialEq)]
pub enum AugAssignOp {
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
    /// Augmented assignment (x += 1, x -= 1, etc.)
    AugAssign {
        target: String,
        op: AugAssignOp,
        value: Expr,
    },
    /// Tuple unpacking assignment (a, b = func() or head, *tail = values)
    TupleAssign {
        targets: Vec<String>,
        value: Expr,
        /// If Some(i), targets[i] is a starred target (*tail)
        starred_index: Option<usize>,
    },
    /// Index swap (a[i], a[j] = a[j], a[i])
    IndexSwap {
        left_targets: Vec<Expr>,
        right_values: Vec<Expr>,
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
    While { condition: Expr, body: Vec<Stmt> },
    /// Return statement
    Return(Option<Expr>),
    /// Expression statement
    Expr(Expr),
    /// Class definition (dataclass -> struct, or class with methods)
    ClassDef {
        name: String,
        fields: Vec<Field>,
        methods: Vec<MethodDef>,
    },
    /// Try-except statement
    TryExcept {
        try_body: Vec<Stmt>,
        except_type: Option<String>,
        except_body: Vec<Stmt>,
    },
    /// Raise statement
    Raise {
        exception_type: String,
        message: Expr,
    },
    /// Import statement (import x as y, from x import y)
    Import {
        module: String,
        alias: Option<String>,
        items: Option<Vec<String>>, // for "from x import a, b, c"
    },
    /// Break statement
    Break,
    /// Continue statement
    Continue,
    /// Assert statement (V1.3.0)
    Assert { test: Expr, msg: Option<Expr> },
}

/// Function parameter with optional default value
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_hint: Option<TypeHint>,
    pub default: Option<Expr>,
    pub variadic: bool,
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

/// Class method
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    pub name: String,
    pub params: Vec<Param>, // Excludes 'self'
    pub return_type: Option<TypeHint>,
    pub body: Vec<Stmt>,
    pub is_static: bool, // @staticmethod
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
        if let Stmt::Assign {
            target,
            type_hint,
            value,
        } = stmt
        {
            assert_eq!(target, "x");
            assert!(type_hint.is_some());
            assert_eq!(value, Expr::IntLiteral(10));
        }
    }
}
