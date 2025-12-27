//! Lexer module - Tokenization

mod token;

pub use token::*;

/// Tokenize Python source code
pub fn tokenize(_source: &str) -> Vec<Token> {
    // TODO: Implement tokenizer
    // For now, pest will handle this directly
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }
}
