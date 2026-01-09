//! PythonBridge - 常駐 Python ワーカーとの通信 (V1.7.0 Enhanced)
//!
//! Rust バイナリから Python ワーカープロセスを起動し、
//! stdin/stdout で NDJSON 通信を行う。

pub mod module_table;
pub mod strategies;
pub mod tsuchinoko_error;
pub mod bridge_error;
pub mod protocol;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

use self::bridge_error::BridgeError;
use self::protocol::{Command as BridgeCmd, Response, TnkValue};

/// 埋め込み Python ワーカーコード
const WORKER_CODE: &str = include_str!("python/v1_7_0_worker.py");

/// Python ワーカーとの通信を管理する構造体
pub struct PythonBridge {
    process: Child,
    request_id: AtomicU64,
    session_id: String,
}

impl PythonBridge {
    /// Python ワーカーを起動
    pub fn new() -> Result<Self, BridgeError> {
        let process = Command::new("python3") // Ensure python3
            .args(["-u", "-c", WORKER_CODE])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| BridgeError::Io(e))?;

        Ok(Self {
            process,
            request_id: AtomicU64::new(1),
            session_id: Uuid::new_v4().to_string(),
        })
    }

    /// 汎用リクエスト送信
    fn send_command(&mut self, cmd: BridgeCmd) -> Result<TnkValue, BridgeError> {
        // リクエスト送信
        let stdin = self.process.stdin.as_mut().ok_or(BridgeError::Unknown("Failed to get stdin".into()))?;
        let json_req = serde_json::to_string(&cmd)?;
        writeln!(stdin, "{}", json_req)?;
        stdin.flush()?;

        // レスポンス受信
        let stdout = self.process.stdout.as_mut().ok_or(BridgeError::Unknown("Failed to get stdout".into()))?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        if line.is_empty() {
             return Err(BridgeError::WorkerCrash("Worker closed stdout (EOF)".into()));
        }

        let response: Response = serde_json::from_str(&line)?;

        match response {
            Response::Ok { value, .. } => Ok(value),
            Response::Error { error, .. } => Err(BridgeError::from_api_error(
                &error.code,
                error.message,
                error.py_type,
                error.traceback
            )),
        }
    }

    // --- V1.7.0 New APIs ---

    pub fn call_function(&mut self, target: &str, args: Vec<TnkValue>) -> Result<TnkValue, BridgeError> {
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::CallFunction {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target.to_string(),
            args,
        })
    }

    pub fn call_method(&mut self, target_handle: &str, method: &str, args: Vec<TnkValue>) -> Result<TnkValue, BridgeError> {
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::CallMethod {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_handle.to_string(),
            method: method.to_string(),
            args,
        })
    }
    
    pub fn get_attribute(&mut self, target_handle: &str, name: &str) -> Result<TnkValue, BridgeError> {
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::GetAttribute {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_handle.to_string(),
            name: name.to_string(),
        })
    }

    pub fn get_item(&mut self, target_handle: &str, key: TnkValue) -> Result<TnkValue, BridgeError> {
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::GetItem {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_handle.to_string(),
            key,
        })
    }
    
    pub fn slice(&mut self, target_handle: &str, start: TnkValue, stop: TnkValue, step: TnkValue) -> Result<TnkValue, BridgeError> {
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::Slice {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_handle.to_string(),
            start,
            stop,
            step,
        })
    }

    pub fn shutdown(&mut self) -> Result<(), BridgeError> {
        // Just kill it or let it die on stdin close (Worker doesn't have shutdown cmd? Checks `op=="shutdown"`, but new one checks main loop)
        // v1_7_0_worker.py terminates on empty line (EOF) or invalid JSON?
        // Let's rely on Drop or a manual kill.
        // Actually, let's close stdin.
        if let Some(_stdin) = self.process.stdin.as_mut() {
            // New worker doesn't implement specific "shutdown" command yet in DISPATCHER?
            // "delete" is there. 
            // `main()` loop: `for line in sys.stdin`. So closing stdin terminates loop.
        }
        self.process.kill().map_err(|e| BridgeError::Io(e))?;
        Ok(())
    }
}

impl Drop for PythonBridge {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
