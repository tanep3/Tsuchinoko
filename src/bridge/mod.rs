//! PythonBridge - 常駐 Python ワーカーとの通信 (V1.7.0 Enhanced)
//!
//! Rust バイナリから Python ワーカープロセスを起動し、
//! stdin/stdout で NDJSON 通信を行う。

pub mod module_table;
pub mod strategies;
pub mod tsuchinoko_error;
pub mod bridge_error;
pub mod protocol;
pub mod type_inference;


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

    fn extract_id(&self, target: &TnkValue) -> Result<String, BridgeError> {
        match target {
            TnkValue::Handle { id, .. } => Ok(id.clone()),
            _ => Err(BridgeError::TypeMismatch(format!("Target object is not a handle (Remote Object). Got: {:?}", target))),
        }
    }

    pub fn call_function(&mut self, target: &str, args: Vec<TnkValue>) -> Result<TnkValue, BridgeError> {
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::CallFunction {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target.to_string(),
            args,
        })
    }

    pub fn call_method(&mut self, target: &TnkValue, method: &str, args: &[TnkValue]) -> Result<TnkValue, BridgeError> {
        let target_id = self.extract_id(target)?;
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::CallMethod {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_id,
            method: method.to_string(),
            args: args.to_vec(),
        })
    }
    
    pub fn get_attribute(&mut self, target: &TnkValue, name: &str) -> Result<TnkValue, BridgeError> {
        let target_id = self.extract_id(target)?;
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::GetAttribute {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_id,
            name: name.to_string(),
        })
    }

    pub fn get_item(&mut self, target: &TnkValue, key: &TnkValue) -> Result<TnkValue, BridgeError> {
        let target_id = self.extract_id(target)?;
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::GetItem {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_id,
            key: key.clone(),
        })
    }
    
    pub fn slice(&mut self, target: &TnkValue, start: TnkValue, stop: TnkValue, step: TnkValue) -> Result<TnkValue, BridgeError> {
        let target_id = self.extract_id(target)?;
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::Slice {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_id,
            start,
            stop,
            step,
        })
    }

    pub fn shutdown(&mut self) -> Result<(), BridgeError> {
        self.process.kill().map_err(|e| BridgeError::Io(e))?;
        Ok(())
    }
}

pub fn display_value(value: &TnkValue) -> String {
    value.to_string()
}

// Compatibility layer for Emitter's generic calls
impl PythonBridge {
    pub fn call_json<T: serde::de::DeserializeOwned>(&mut self, target: &str, args: &[serde_json::Value]) -> Result<T, BridgeError> {
        let tnk_args: Vec<TnkValue> = args.iter().map(|v| crate::bridge::type_inference::from_value(v.clone())).collect();
        // call_function for module functions? or call_method?
        // Emitter uses call_json("numpy.matmul") -> Module function.
        let result = self.call_function(target, tnk_args)?;
        let json_val: serde_json::Value = result.into();
        serde_json::from_value(json_val).map_err(BridgeError::Json)
    }

    pub fn call_json_method<T: serde::de::DeserializeOwned>(
        &mut self,
        handle: serde_json::Value, // Old Emitter passes Handle as JSON Value?
        method: &str,
        args: &[serde_json::Value],
    ) -> Result<T, BridgeError> {
        // Handle might be { "__handle__": "id" } or just TnkValue::Handle
        // Convert handle
        let target: TnkValue = crate::bridge::type_inference::from_value(handle);
        let tnk_args: Vec<TnkValue> = args.iter().map(|v| crate::bridge::type_inference::from_value(v.clone())).collect();
        let result = self.call_method(&target, method, &tnk_args)?;
        let json_val: serde_json::Value = result.into();
        serde_json::from_value(json_val).map_err(BridgeError::Json)
    }
}

impl Drop for PythonBridge {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
