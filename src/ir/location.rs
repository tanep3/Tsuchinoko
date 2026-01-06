//! Source Location Definitions
//!
//! ソースコード位置情報を定義する。
//! コンパイルパイプライン全体で一貫して行番号情報を保持するための型。

/// ソースコード位置情報（全コンパイルステージで共有）
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceLocation {
    /// 行番号 (1-indexed)
    pub line: usize,
    /// 列番号 (1-indexed)
    pub column: usize,
    /// ファイル名（将来の複数ファイル対応用）
    pub file: Option<String>,
}

impl SourceLocation {
    /// 新しい SourceLocation を作成
    pub fn new(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            file: None,
        }
    }

    /// ファイル名付きの SourceLocation を作成
    pub fn with_file(line: usize, column: usize, file: String) -> Self {
        Self {
            line,
            column,
            file: Some(file),
        }
    }

    /// 位置が不明な場合の SourceLocation
    pub fn unknown() -> Self {
        Self::default()
    }

    /// 位置情報があるかどうか
    pub fn is_known(&self) -> bool {
        self.line > 0
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.is_known() {
            return Ok(());
        }
        
        if let Some(ref file) = self.file {
            write!(f, "[{}:{}]", file, self.line)
        } else {
            write!(f, "[line {}]", self.line)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_new() {
        let loc = SourceLocation::new(10, 5);
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
        assert!(loc.file.is_none());
    }

    #[test]
    fn test_source_location_with_file() {
        let loc = SourceLocation::with_file(15, 1, "main.py".to_string());
        assert_eq!(loc.line, 15);
        assert_eq!(loc.file, Some("main.py".to_string()));
    }

    #[test]
    fn test_source_location_display() {
        let loc = SourceLocation::new(10, 1);
        assert_eq!(format!("{}", loc), "[line 10]");

        let loc_with_file = SourceLocation::with_file(15, 1, "main.py".to_string());
        assert_eq!(format!("{}", loc_with_file), "[main.py:15]");
    }

    #[test]
    fn test_source_location_unknown() {
        let loc = SourceLocation::unknown();
        assert!(!loc.is_known());
        assert_eq!(format!("{}", loc), "");
    }
}
