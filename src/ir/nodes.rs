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
    Assign { target: String, value: Box<IrExpr> },
    /// Index assignment (arr[i] = val)
    IndexAssign {
        target: Box<IrExpr>,
        index: Box<IrExpr>,
        value: Box<IrExpr>,
    },
    /// Augmented assignment (x += 1, etc.)
    AugAssign {
        target: String,
        op: IrAugAssignOp,
        value: Box<IrExpr>,
    },
    /// Multiple assignment (a, b = val) - used for tuple unpacking
    MultiAssign {
        targets: Vec<String>,
        value: Box<IrExpr>,
    },
    /// Multiple variable declaration (let (a, b) = val)
    MultiVarDecl {
        targets: Vec<(String, Type, bool)>, // (name, type, mutable)
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
    /// Field assignment (self.field = value)
    FieldAssign {
        target: Box<IrExpr>, // Usually IrExpr::Var("self")
        field: String,
        value: Box<IrExpr>,
    },
    /// Struct definition (from @dataclass)
    StructDef {
        name: String,
        fields: Vec<(String, Type)>,
    },
    /// Impl block for methods
    ImplBlock {
        struct_name: String,
        methods: Vec<IrNode>, // Contains MethodDecl nodes
    },
    /// Method declaration inside impl block
    MethodDecl {
        name: String,
        params: Vec<(String, Type)>, // Excludes &self
        ret: Type,
        body: Vec<IrNode>,
        takes_self: bool,     // true for instance methods, false for static
        takes_mut_self: bool, // true if method modifies self (field assignment)
    },
    /// Try-except block (maps to match Result)
    TryBlock {
        try_body: Vec<IrNode>,
        except_body: Vec<IrNode>,
    },
    /// Type alias (type Alias = T)
    TypeAlias { name: String, ty: Type },
    /// Panic (from raise)
    Panic(String),
    /// Break statement
    Break,
    /// Continue statement
    Continue,
    /// Sequence of nodes (for returning multiple top-level items like StructDef + ImplBlock)
    Sequence(Vec<IrNode>),
}

/// IR expression types
#[derive(Debug, Clone)]
pub enum IrExpr {
    /// Literal values
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    /// None literal (Rust None)
    NoneLit,
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
        func: Box<IrExpr>,
        args: Vec<IrExpr>,
    },
    /// Closure (lambda / nested function)
    Closure {
        params: Vec<String>,
        body: Vec<IrNode>,
        ret_type: Type,
    },
    /// List/Vec literal
    List {
        elem_type: Type,
        elements: Vec<IrExpr>,
    },
    /// Tuple literal
    Tuple(Vec<IrExpr>),
    /// List comprehension [elt for target in iter if condition]
    ListComp {
        elt: Box<IrExpr>,
        target: String,
        iter: Box<IrExpr>,
        condition: Option<Box<IrExpr>>,
    },
    /// Index access
    Index {
        target: Box<IrExpr>,
        index: Box<IrExpr>,
    },
    /// Slice access (target[start..end])
    Slice {
        target: Box<IrExpr>,
        start: Option<Box<IrExpr>>,
        end: Option<Box<IrExpr>>,
    },
    /// Range (for loops)
    Range {
        start: Box<IrExpr>,
        end: Box<IrExpr>,
    },
    /// Method call (e.g., arr.len())
    MethodCall {
        target: Box<IrExpr>,
        method: String,
        args: Vec<IrExpr>,
    },
    /// Field access (e.g., obj.field)
    FieldAccess {
        target: Box<IrExpr>,
        field: String,
    },
    /// Reference (&expr)
    Reference {
        target: Box<IrExpr>,
    },
    /// Mutable Reference (&mut expr)
    MutReference {
        target: Box<IrExpr>,
    },
    /// Dict/HashMap literal
    Dict {
        key_type: Type,
        value_type: Type,
        entries: Vec<(IrExpr, IrExpr)>,
    },
    /// f-string (format! macro)
    FString {
        parts: Vec<String>,
        values: Vec<IrExpr>,
    },
    /// Conditional Expression (if test { body } else { orelse })
    IfExp {
        test: Box<IrExpr>,
        body: Box<IrExpr>,
        orelse: Box<IrExpr>,
    },
    /// Box::new helper
    BoxNew(Box<IrExpr>),
    /// Explicit cast (expr as type)
    Cast {
        target: Box<IrExpr>,
        ty: String,
    },
    /// Raw Rust code (for patterns that don't have IR equivalents)
    RawCode(String),
}

/// IR binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum IrBinOp {
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
    Contains, // x in dict -> dict.contains_key(&x)
    Is,       // x is None -> x.is_none()
    IsNot,    // x is not None -> x.is_some()
}

/// IR unary operators
#[derive(Debug, Clone)]
pub enum IrUnaryOp {
    Neg,
    Not,
    Deref, // *expr
}

/// IR augmented assignment operators
#[derive(Debug, Clone)]
pub enum IrAugAssignOp {
    Add,      // +=
    Sub,      // -=
    Mul,      // *=
    Div,      // /=
    FloorDiv, // //=
    Mod,      // %=
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
