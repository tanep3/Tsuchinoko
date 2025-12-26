//! Token definitions

/// Token types for Python lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),

    // Identifiers and keywords
    Ident(String),
    Keyword(Keyword),

    // Operators
    Operator(Operator),

    // Delimiters
    Delimiter(Delimiter),

    // Indentation
    Indent,
    Dedent,
    Newline,

    // End of file
    Eof,
}

/// Python keywords
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Def,
    Return,
    If,
    Elif,
    Else,
    For,
    While,
    In,
    And,
    Or,
    Not,
    True,
    False,
    None,
    Class,
    Try,
    Except,
    Finally,
    Raise,
    Import,
    From,
    As,
    Pass,
    Break,
    Continue,
}

/// Operators
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    // Arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    DoubleSlash,
    Percent,
    DoubleStar,

    // Comparison
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // Assignment
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,

    // Other
    Arrow,  // ->
    Colon,
}

/// Delimiters
#[derive(Debug, Clone, PartialEq)]
pub enum Delimiter {
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Dot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_equality() {
        let t1 = Token::IntLiteral(42);
        let t2 = Token::IntLiteral(42);
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_keyword_equality() {
        assert_eq!(Keyword::Def, Keyword::Def);
        assert_ne!(Keyword::Def, Keyword::Return);
    }
}
