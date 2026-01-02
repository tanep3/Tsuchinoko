//! Import Strategy Trait Definition
//!
//! ImportStrategy トレイトは、Pythonモジュールのimport方式を抽象化する。
//! 各実装（Native, Resident）がこのトレイトを実装する。

pub mod native;
pub mod resident;

pub use native::NativeStrategy;
pub use resident::ResidentStrategy;

/// インポート方式を表すトレイト
///
/// Pythonのimport文をRustコードに変換する際の戦略を定義。
/// 各実装は特定の方式でのコード生成を担当する。
pub trait ImportStrategy {
    /// この戦略がサポートするターゲットかどうか
    ///
    /// # Arguments
    /// * `target` - チェック対象（例: "math.sqrt", "numpy.mean"）
    ///
    /// # Returns
    /// サポートする場合true
    fn supports(&self, target: &str) -> bool;

    /// Rustコードを生成
    ///
    /// # Arguments
    /// * `target` - ターゲット（例: "math.sqrt"）
    /// * `args` - 引数のRust表現
    ///
    /// # Returns
    /// 生成されたRustコード。生成できない場合はNone
    fn generate_code(&self, target: &str, args: &[String]) -> Option<String>;

    /// 戦略の名前を取得
    fn name(&self) -> &'static str;

    /// 優先度を取得（数値が小さいほど優先）
    fn priority(&self) -> u8;
}

/// import方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportMode {
    /// Rust ネイティブ実装（最高速）
    Native,
    /// PyO3 経由（動作確認済みのみ）
    PyO3,
    /// 常駐 Python プロセス（fallback）
    Resident,
}

#[cfg(test)]
mod tests {
    use super::*;

    // モック実装でトレイト定義のテスト
    struct MockStrategy;

    impl ImportStrategy for MockStrategy {
        fn supports(&self, target: &str) -> bool {
            target.starts_with("mock.")
        }

        fn generate_code(&self, target: &str, args: &[String]) -> Option<String> {
            if self.supports(target) {
                Some(format!("mock_call({}, {:?})", target, args))
            } else {
                None
            }
        }

        fn name(&self) -> &'static str {
            "Mock"
        }

        fn priority(&self) -> u8 {
            99
        }
    }

    #[test]
    fn test_mock_strategy_supports() {
        let strategy = MockStrategy;
        assert!(strategy.supports("mock.func"));
        assert!(!strategy.supports("other.func"));
    }

    #[test]
    fn test_mock_strategy_generate() {
        let strategy = MockStrategy;
        let result = strategy.generate_code("mock.func", &["arg1".to_string()]);
        assert!(result.is_some());
    }
}
