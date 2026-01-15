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

/// Native 実装のバインディング形式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeBinding {
    /// Rust のメソッド呼び出し (obj.method(args))
    Method(&'static str),
    /// Rust の定数 (std::f64::consts::PI)
    Constant(&'static str),
}

/// target 文字列からバインディング情報を取得
pub fn get_native_binding(target: &str) -> Option<NativeBinding> {
    match target {
        "math.sqrt" => Some(NativeBinding::Method("sqrt")),
        "math.sin" => Some(NativeBinding::Method("sin")),
        "math.cos" => Some(NativeBinding::Method("cos")),
        "math.tan" => Some(NativeBinding::Method("tan")),
        "math.asin" => Some(NativeBinding::Method("asin")),
        "math.acos" => Some(NativeBinding::Method("acos")),
        "math.atan" => Some(NativeBinding::Method("atan")),
        "math.floor" => Some(NativeBinding::Method("floor")),
        "math.ceil" => Some(NativeBinding::Method("ceil")),
        "math.abs" => Some(NativeBinding::Method("abs")),
        "math.pow" => Some(NativeBinding::Method("powf")),
        "math.log" => Some(NativeBinding::Method("ln")),
        "math.log10" => Some(NativeBinding::Method("log10")),
        "math.log2" => Some(NativeBinding::Method("log2")),
        "math.exp" => Some(NativeBinding::Method("exp")),
        "math.round" => Some(NativeBinding::Method("round")),
        "math.pi" => Some(NativeBinding::Constant("std::f64::consts::PI")),
        "math.e" => Some(NativeBinding::Constant("std::f64::consts::E")),
        "math.tau" => Some(NativeBinding::Constant("std::f64::consts::TAU")),
        "math.inf" => Some(NativeBinding::Constant("f64::INFINITY")),
        "math.nan" => Some(NativeBinding::Constant("f64::NAN")),
        _ => None,
    }
}

/// target 文字列から方式を決定
pub fn get_import_mode(target: &str) -> ImportMode {
    // Native 実装済み target
    if is_native_target(target) {
        return ImportMode::Native;
    }

    // モジュール単位での判定
    if let Some(module) = target.split('.').next() {
        if is_native_module(module) {
            return ImportMode::Native;
        }
    }

    // PyO3 で動作確認済み target（現在は空）
    if is_pyo3_target(target) {
        return ImportMode::PyO3;
    }

    // デフォルトは常駐プロセス
    ImportMode::Resident
}

/// モジュール単位で Native (Rust 内製) かどうかを判定
pub fn is_native_module(module: &str) -> bool {
    matches!(module, "math" | "typing")
}

/// Native 実装済みかどうか (関数/属性レベル)
pub fn is_native_target(target: &str) -> bool {
    get_native_binding(target).is_some()
}

/// PyO3 で動作確認済みかどうか
fn is_pyo3_target(_target: &str) -> bool {
    // V1.2.0 では PyO3 は ctypes 問題があるため空
    // 将来、ctypes を使わないモジュールを追加予定
    false
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
            get_native_binding("math.sqrt"),
            Some(NativeBinding::Method("sqrt"))
        );
        assert_eq!(
            get_native_binding("math.pow"),
            Some(NativeBinding::Method("powf"))
        );
    }
}
