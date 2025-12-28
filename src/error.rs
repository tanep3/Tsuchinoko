//! Error types for Tsuchinoko transpiler

use thiserror::Error;

/// Main error type for Tsuchinoko
#[derive(Debug, Error)]
pub enum TsuchinokoError {
    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("Type error at line {line}: {message}")]
    TypeError { line: usize, message: String },

    #[error("Undefined variable '{name}' at line {line}")]
    UndefinedVariable { name: String, line: usize },

    #[error("Unsupported syntax at line {line}: {syntax}")]
    UnsupportedSyntax { syntax: String, line: usize },

    #[error("Semantic error: {message}")]
    SemanticError { message: String },

    #[error("Compile error: {0}")]
    CompileError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TsuchinokoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_display() {
        let err = TsuchinokoError::ParseError {
            line: 5,
            message: "unexpected token".to_string(),
        };
        assert_eq!(format!("{err}"), "Parse error at line 5: unexpected token");
    }

    #[test]
    fn test_type_error_display() {
        let err = TsuchinokoError::TypeError {
            line: 10,
            message: "expected int, got str".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Type error at line 10: expected int, got str"
        );
    }
}
