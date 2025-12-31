//! PythonBridge - 常駐 Python ワーカーとの通信
//!
//! Rust バイナリから Python ワーカープロセスを起動し、
//! stdin/stdout で NDJSON 通信を行う。

pub mod module_table;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// 埋め込み Python ワーカーコード
const WORKER_CODE: &str = include_str!("worker.py");

/// Python ワーカーとの通信を管理する構造体
pub struct PythonBridge {
    process: Child,
    request_id: AtomicU64,
}

impl PythonBridge {
    /// Python ワーカーを起動
    pub fn new() -> Result<Self, String> {
        let process = Command::new("python")
            .args(["-u", "-c", WORKER_CODE])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn Python worker: {}", e))?;

        Ok(Self {
            process,
            request_id: AtomicU64::new(1),
        })
    }

    /// リクエストを送信してレスポンスを受信
    fn call_raw(&mut self, target: &str, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = serde_json::json!({
            "id": id,
            "op": "call",
            "target": target,
            "args": args,
        });

        // リクエスト送信
        let stdin = self.process.stdin.as_mut()
            .ok_or("Failed to get stdin")?;
        writeln!(stdin, "{}", request.to_string())
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        stdin.flush()
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;

        // レスポンス受信
        let stdout = self.process.stdout.as_mut()
            .ok_or("Failed to get stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)
            .map_err(|e| format!("Failed to read from stdout: {}", e))?;

        let response: serde_json::Value = serde_json::from_str(&line)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if response["ok"].as_bool() == Some(true) {
            Ok(response["result"].clone())
        } else {
            Err(format!("Python error: {}", response["error"]))
        }
    }

    /// i64 を返す呼び出し
    pub fn call_i64(&mut self, target: &str, args: &[serde_json::Value]) -> Result<i64, String> {
        let result = self.call_raw(target, args)?;
        result.as_i64().ok_or_else(|| format!("Expected i64, got: {}", result))
    }

    /// f64 を返す呼び出し
    pub fn call_f64(&mut self, target: &str, args: &[serde_json::Value]) -> Result<f64, String> {
        let result = self.call_raw(target, args)?;
        result.as_f64().ok_or_else(|| format!("Expected f64, got: {}", result))
    }

    /// String を返す呼び出し
    pub fn call_string(&mut self, target: &str, args: &[serde_json::Value]) -> Result<String, String> {
        let result = self.call_raw(target, args)?;
        result.as_str().map(|s| s.to_string()).ok_or_else(|| format!("Expected string, got: {}", result))
    }

    /// Vec<i64> を返す呼び出し
    pub fn call_vec_i64(&mut self, target: &str, args: &[serde_json::Value]) -> Result<Vec<i64>, String> {
        let result = self.call_raw(target, args)?;
        result.as_array()
            .ok_or_else(|| format!("Expected array, got: {}", result))?
            .iter()
            .map(|v| v.as_i64().ok_or_else(|| format!("Expected i64 in array, got: {}", v)))
            .collect()
    }

    /// Vec<f64> を返す呼び出し
    pub fn call_vec_f64(&mut self, target: &str, args: &[serde_json::Value]) -> Result<Vec<f64>, String> {
        let result = self.call_raw(target, args)?;
        result.as_array()
            .ok_or_else(|| format!("Expected array, got: {}", result))?
            .iter()
            .map(|v| v.as_f64().ok_or_else(|| format!("Expected f64 in array, got: {}", v)))
            .collect()
    }

    /// JSON を返す呼び出し（汎用、自動変換サポート）
    pub fn call_json<T: serde::de::DeserializeOwned>(&mut self, target: &str, args: &[serde_json::Value]) -> Result<T, String> {
        let result = self.call_raw(target, args)?;
        serde_json::from_value(result).map_err(|e| format!("Type conversion failed: {}", e))
    }

    /// ping テスト
    pub fn ping(&mut self) -> Result<bool, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "id": id,
            "op": "ping",
        });

        let stdin = self.process.stdin.as_mut()
            .ok_or("Failed to get stdin")?;
        writeln!(stdin, "{}", request.to_string())
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        stdin.flush()
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;

        let stdout = self.process.stdout.as_mut()
            .ok_or("Failed to get stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)
            .map_err(|e| format!("Failed to read from stdout: {}", e))?;

        let response: serde_json::Value = serde_json::from_str(&line)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(response["ok"].as_bool() == Some(true) && response["result"] == "pong")
    }

    /// ワーカーを終了
    pub fn shutdown(&mut self) -> Result<(), String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "id": id,
            "op": "shutdown",
        });

        if let Some(stdin) = self.process.stdin.as_mut() {
            let _ = writeln!(stdin, "{}", request.to_string());
            let _ = stdin.flush();
        }

        self.process.wait().map_err(|e| format!("Failed to wait for worker: {}", e))?;
        Ok(())
    }
}

impl Drop for PythonBridge {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
