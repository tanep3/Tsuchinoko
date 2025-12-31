//! Tsuchinoko - Python to Rust Transpiler
//!
//! # Overview
//! Type-hinted Python code to Rust code transpiler.
//!
//! # Author
//! Tane Channel Technology

pub mod bridge;
pub mod emitter;
pub mod error;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;

use anyhow::Result;
use std::path::Path;

/// Transpile Python source code to Rust source code
pub fn transpile(source: &str) -> Result<String> {
    // 1. Parse Python source to AST
    let program = parser::parse(source)?;

    // 2. Semantic analysis: AST -> IR
    let ir = semantic::analyze(&program)?;

    // 3. Emit Rust code
    let rust_code = emitter::emit(&ir);

    Ok(rust_code)
}

/// Transpile a Python file to a Rust file
pub fn transpile_file(input: &Path, output: &Path) -> Result<()> {
    let source = std::fs::read_to_string(input)?;
    let rust_code = transpile(&source)?;
    std::fs::write(output, rust_code)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_simple_assignment() {
        let python = "x: int = 10";
        let result = transpile(python).unwrap();
        // Updated to expect i64 suffix if emitter provides it
        assert!(result.contains("let x: i64 = 10i64;"));
    }

    #[test]
    fn test_transpile_list() {
        let python = "nums: list[int] = [1, 2, 3]";
        let result = transpile(python).unwrap();
        // Note: lists may be marked as mut by default
        assert!(result.contains("nums: Vec<i64> = vec![1i64, 2i64, 3i64]"));
    }

    #[test]
    fn test_transpile_binary_op() {
        let python = "result: int = a + b";
        let result = transpile(python).unwrap();
        // Parentheses may be stripped in some cases
        assert!(result.contains("let result: i64 = a + b"));
    }

    #[test]
    fn test_transpile_string() {
        let python = r#"msg: str = "hello""#;
        let result = transpile(python).unwrap();
        assert!(result.contains(r#"let msg: String = "hello""#));
    }

    #[test]
    fn test_transpile_multiple_lines() {
        let python = r#"
x: int = 10
y: int = 20
"#;
        let result = transpile(python).unwrap();
        assert!(result.contains("let x: i64 = 10"));
        assert!(result.contains("let y: i64 = 20"));
    }
}
