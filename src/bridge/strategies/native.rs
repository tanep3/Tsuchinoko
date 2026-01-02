//! Native Implementation Strategy
//!
//! PythonモジュールをRustネイティブコードに変換する戦略。
//! mathモジュールなど、Rustのf64メソッドで直接実装可能なものを扱う。

use super::ImportStrategy;

/// Native実装戦略
///
/// mathモジュールなど、Rustで直接実装可能な関数を変換する。
/// 最高速のパフォーマンスを提供。
pub struct NativeStrategy;

impl NativeStrategy {
    /// 新しいNativeStrategyを作成
    pub fn new() -> Self {
        Self
    }
}

impl Default for NativeStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportStrategy for NativeStrategy {
    fn supports(&self, target: &str) -> bool {
        matches!(
            target,
            "math.sqrt"
                | "math.sin"
                | "math.cos"
                | "math.tan"
                | "math.floor"
                | "math.ceil"
                | "math.abs"
                | "math.pow"
                | "math.log"
                | "math.log10"
                | "math.exp"
        )
    }

    fn generate_code(&self, target: &str, args: &[String]) -> Option<String> {
        match target {
            "math.sqrt" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).sqrt()"))
            }
            "math.sin" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).sin()"))
            }
            "math.cos" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).cos()"))
            }
            "math.tan" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).tan()"))
            }
            "math.floor" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).floor()"))
            }
            "math.ceil" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).ceil()"))
            }
            "math.abs" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).abs()"))
            }
            "math.pow" => {
                let base = args.first()?;
                let exp = args.get(1)?;
                Some(format!("({base} as f64).powf({exp} as f64)"))
            }
            "math.log" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).ln()"))
            }
            "math.log10" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).log10()"))
            }
            "math.exp" => {
                let arg = args.first()?;
                Some(format!("({arg} as f64).exp()"))
            }
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "Native"
    }

    fn priority(&self) -> u8 {
        0 // 最高優先度
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_supports_math() {
        let strategy = NativeStrategy::new();
        assert!(strategy.supports("math.sqrt"));
        assert!(strategy.supports("math.sin"));
        assert!(strategy.supports("math.pow"));
    }

    #[test]
    fn test_native_does_not_support_numpy() {
        let strategy = NativeStrategy::new();
        assert!(!strategy.supports("numpy.mean"));
        assert!(!strategy.supports("pandas.DataFrame"));
    }

    #[test]
    fn test_native_generate_sqrt() {
        let strategy = NativeStrategy::new();
        let result = strategy.generate_code("math.sqrt", &["x".to_string()]);
        assert_eq!(result, Some("(x as f64).sqrt()".to_string()));
    }

    #[test]
    fn test_native_generate_pow() {
        let strategy = NativeStrategy::new();
        let result = strategy.generate_code("math.pow", &["x".to_string(), "2".to_string()]);
        assert_eq!(result, Some("(x as f64).powf(2 as f64)".to_string()));
    }

    #[test]
    fn test_native_priority() {
        let strategy = NativeStrategy::new();
        assert_eq!(strategy.priority(), 0);
    }
}
