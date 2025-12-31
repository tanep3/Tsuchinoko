//! 方式選択テーブル - import 方式を target 単位で決定
//!
//! 優先順位:
//! 1. Native - Rust ネイティブで実装済み
//! 2. PyO3 - PyO3 経由で動作確認済み（現在は空）
//! 3. Resident - 常駐プロセス（デフォルト fallback）

/// import 方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportMode {
    /// Rust ネイティブ実装（最高速）
    Native,
    /// PyO3 経由（動作確認済みのみ）
    PyO3,
    /// 常駐 Python プロセス（fallback）
    Resident,
}

/// target 文字列から方式を決定
pub fn get_import_mode(target: &str) -> ImportMode {
    // Native 実装済み target
    if is_native_target(target) {
        return ImportMode::Native;
    }

    // PyO3 で動作確認済み target（現在は空）
    if is_pyo3_target(target) {
        return ImportMode::PyO3;
    }

    // デフォルトは常駐プロセス
    ImportMode::Resident
}

/// Native 実装済みかどうか
fn is_native_target(target: &str) -> bool {
    matches!(
        target,
        // math モジュール（Rust の f64 メソッドで実装）
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

/// PyO3 で動作確認済みかどうか
fn is_pyo3_target(_target: &str) -> bool {
    // V1.2.0 では PyO3 は ctypes 問題があるため空
    // 将来、ctypes を使わないモジュールを追加予定
    false
}

/// Native 実装の Rust コードを生成
pub fn generate_native_code(target: &str, args: &[String]) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_targets() {
        assert_eq!(get_import_mode("math.sqrt"), ImportMode::Native);
        assert_eq!(get_import_mode("math.sin"), ImportMode::Native);
    }

    #[test]
    fn test_resident_fallback() {
        assert_eq!(get_import_mode("numpy.mean"), ImportMode::Resident);
        assert_eq!(get_import_mode("pandas.DataFrame"), ImportMode::Resident);
        assert_eq!(get_import_mode("unknown.function"), ImportMode::Resident);
    }

    #[test]
    fn test_native_code_generation() {
        assert_eq!(
            generate_native_code("math.sqrt", &["x".to_string()]),
            Some("(x as f64).sqrt()".to_string())
        );
        assert_eq!(
            generate_native_code("math.pow", &["x".to_string(), "2".to_string()]),
            Some("(x as f64).powf(2 as f64)".to_string())
        );
    }
}
