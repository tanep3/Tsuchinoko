//! Resident Implementation Strategy
//!
//! 常駐Pythonプロセス経由でのコード生成を行う戦略。
//! NumPy, Pandas等、Rustで直接実装できないモジュールを扱う。
//! デフォルトのフォールバック戦略。

use super::ImportStrategy;

/// Resident実装戦略
///
/// py_bridge経由で常駐Pythonプロセスを呼び出すコードを生成。
/// Native/PyO3でサポートされないすべてのターゲットを受け入れる。
pub struct ResidentStrategy;

impl ResidentStrategy {
    /// 新しいResidentStrategyを作成
    pub fn new() -> Self {
        Self
    }
}

impl Default for ResidentStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportStrategy for ResidentStrategy {
    fn supports(&self, _target: &str) -> bool {
        // フォールバックとして全てをサポート
        true
    }

    fn generate_code(&self, target: &str, args: &[String]) -> Option<String> {
        // py_bridge.call_json() 呼び出しを生成
        let args_json = args
            .iter()
            .map(|a| format!("serde_json::json!({})", a))
            .collect::<Vec<_>>()
            .join(", ");

        Some(format!(
            "py_bridge.call_json::<serde_json::Value>(\"{}\", &[{}]).unwrap()",
            target, args_json
        ))
    }

    fn name(&self) -> &'static str {
        "Resident"
    }

    fn priority(&self) -> u8 {
        255 // 最低優先度（フォールバック）
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resident_supports_all() {
        let strategy = ResidentStrategy::new();
        assert!(strategy.supports("numpy.mean"));
        assert!(strategy.supports("pandas.DataFrame"));
        assert!(strategy.supports("any.module.function"));
    }

    #[test]
    fn test_resident_generate_code() {
        let strategy = ResidentStrategy::new();
        let result = strategy.generate_code("numpy.mean", &["arr".to_string()]);
        assert!(result.is_some());
        let code = result.unwrap();
        assert!(code.contains("py_bridge.call_json"));
        assert!(code.contains("numpy.mean"));
    }

    #[test]
    fn test_resident_generate_with_multiple_args() {
        let strategy = ResidentStrategy::new();
        let result = strategy.generate_code("numpy.add", &["a".to_string(), "b".to_string()]);
        let code = result.unwrap();
        assert!(code.contains("serde_json::json!(a)"));
        assert!(code.contains("serde_json::json!(b)"));
    }

    #[test]
    fn test_resident_priority() {
        let strategy = ResidentStrategy::new();
        assert_eq!(strategy.priority(), 255);
    }
}
