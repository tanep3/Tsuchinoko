//! TsuchinokoError - Python例外を表現するエラー型
//!
//! このファイルは**生成されるRustコード**に埋め込まれる型定義です。
//! トランスパイラ内部のエラー型（crate::error::TsuchinokoError）とは別物です。

/// 生成コードに埋め込む TsuchinokoError 型の定義
pub const TSUCHINOKO_ERROR_DEFINITION: &str = r#"
/// Python例外を表現するエラー型
#[derive(Debug, Clone)]
pub struct TsuchinokoError {
    /// 例外の種類 ("ValueError", "RuntimeError" 等)
    pub kind: String,
    /// エラーメッセージ
    pub message: String,
    /// 原因となった例外（raise from 用）
    pub cause: Option<Box<TsuchinokoError>>,
    /// ソースコード行番号（0 = 不明）
    pub line: usize,
}

impl TsuchinokoError {
    /// 新しいエラーを作成
    pub fn new(kind: &str, message: &str, cause: Option<TsuchinokoError>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.to_string(),
            cause: cause.map(Box::new),
            line: 0,
        }
    }
    
    /// 行番号付きでエラーを作成
    pub fn with_line(kind: &str, message: &str, line: usize, cause: Option<TsuchinokoError>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.to_string(),
            cause: cause.map(Box::new),
            line,
        }
    }
    
    /// 内部エラー（panic回収用）を作成
    pub fn internal(message: &str) -> Self {
        Self::new("InternalError", message, None)
    }
}

impl std::fmt::Display for TsuchinokoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 行番号があれば表示
        if self.line > 0 {
            write!(f, "[line {}] ", self.line)?;
        }
        write!(f, "{}: {}", self.kind, self.message)?;
        if let Some(cause) = &self.cause {
            write!(f, "\n  Caused by: {}", cause)?;
        }
        Ok(())
    }
}

impl std::error::Error for TsuchinokoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_ref().map(|c| c.as_ref() as &(dyn std::error::Error + 'static))
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_definition_not_empty() {
        assert!(!TSUCHINOKO_ERROR_DEFINITION.is_empty());
        assert!(TSUCHINOKO_ERROR_DEFINITION.contains("TsuchinokoError"));
        assert!(TSUCHINOKO_ERROR_DEFINITION.contains("cause"));
    }
}
