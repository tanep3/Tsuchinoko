//! PyO3 Implementation Strategy
//!
//! PyO3経由でPythonモジュールを直接呼び出す戦略。
//! ctypes問題が解決された後に本実装を追加予定。

use super::ImportStrategy;

/// PyO3実装戦略
///
/// PyO3経由でPythonインタープリタを埋め込み、
/// Pythonモジュールを直接呼び出す。
/// Nativeより遅いがResidentより高速。
///
/// # 現状
/// V1.2.0では ctypes 問題があるため空実装。
/// 将来、ctypes を使わないモジュールから順次対応予定。
pub struct PyO3Strategy;

impl PyO3Strategy {
    /// 新しいPyO3Strategyを作成
    pub fn new() -> Self {
        Self
    }
}

impl Default for PyO3Strategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportStrategy for PyO3Strategy {
    fn supports(&self, _target: &str) -> bool {
        // V1.2.0: ctypes問題があるため現在は何もサポートしない
        // 将来: numpy, pandas等のctypes不使用APIをここに追加
        false
    }

    fn generate_code(&self, _target: &str, _args: &[String]) -> Option<String> {
        // 現在は何も生成しない
        // 将来: PyO3+pyo3-asyncio による直接呼び出しを生成
        None
    }

    fn name(&self) -> &'static str {
        "PyO3"
    }

    fn priority(&self) -> u8 {
        10 // NativeとResidentの中間
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pyo3_supports_nothing_currently() {
        let strategy = PyO3Strategy::new();
        // 現在は何もサポートしない
        assert!(!strategy.supports("numpy.mean"));
        assert!(!strategy.supports("pandas.DataFrame"));
        assert!(!strategy.supports("any.module"));
    }

    #[test]
    fn test_pyo3_generate_code_returns_none() {
        let strategy = PyO3Strategy::new();
        assert!(strategy.generate_code("numpy.mean", &["arr".to_string()]).is_none());
    }

    #[test]
    fn test_pyo3_priority() {
        let strategy = PyO3Strategy::new();
        // Native (0) < PyO3 (10) < Resident (255)
        assert_eq!(strategy.priority(), 10);
    }

    #[test]
    fn test_pyo3_name() {
        let strategy = PyO3Strategy::new();
        assert_eq!(strategy.name(), "PyO3");
    }
}
