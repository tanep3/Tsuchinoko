//! PythonBridge - 常駐 Python ワーカーとの通信 (V1.7.0 Enhanced)
//!
//! Rust バイナリから Python ワーカープロセスを起動し、
//! stdin/stdout で NDJSON 通信を行う。

// pub mod module_table; // Compiler-only
pub mod strategies;
pub mod tsuchinoko_error;
pub mod bridge_error;
pub mod protocol;
pub mod type_inference;
// pub mod builtin_table; // Compiler-only


use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

use self::bridge_error::BridgeError;
use self::protocol::{Command as BridgeCmd, Response, TnkValue};

/// 埋め込み Python ワーカーコード
const WORKER_CODE: &str = include_str!("python/worker.py");

use std::cell::RefCell;

/// Python ワーカーとの通信を管理する構造体
pub struct PythonBridge {
    // V1.7.0 Option B: Use RefCell for Interior Mutability to allow &self methods
    process: RefCell<Child>,
    request_id: AtomicU64,
    session_id: String,
    // V1.7.0 ModuleTable: alias -> module_name
    modules: RefCell<std::collections::HashMap<String, String>>,
}

impl PythonBridge {
    /// Python ワーカーを起動
    pub fn new() -> Result<Self, BridgeError> {
        let (cmd_name, is_fallback) = if let Ok(path) = std::env::var("PYO3_PYTHON") {
            (path, false)
        } else {
            ("python".to_string(), true)
        };

        eprintln!("[Tsuchinoko] Launching Python Worker with: {}", cmd_name);

        let mut child_result = Command::new(&cmd_name)
            .args(["-u", "-c", WORKER_CODE])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();

        if child_result.is_err() && is_fallback {
            eprintln!("[Tsuchinoko] 'python' failed, trying 'python3'...");
            child_result = Command::new("python3")
                .args(["-u", "-c", WORKER_CODE])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn();
        }

        let process = child_result.map_err(|e| {
            eprintln!("[Tsuchinoko] Failed to launch Python Worker: {:?}", e);
            BridgeError::Io(e)
        })?;

        Ok(Self {
            process: RefCell::new(process),
            request_id: AtomicU64::new(1),
            session_id: Uuid::new_v4().to_string(),
            modules: RefCell::new(std::collections::HashMap::new()),
        })
    }

    /// 汎用リクエスト送信
    /// Uses interior mutability to lock valid only for the duration of the send/recv
    /// Accepts Command with lifetimes
    fn send_command<'a>(&self, cmd: BridgeCmd<'a>) -> Result<TnkValue, BridgeError> {
        let mut process = self.process.borrow_mut();
        
        // リクエスト送信
        let stdin = process.stdin.as_mut().ok_or(BridgeError::Unknown("Failed to get stdin".into()))?;
        let json_req = serde_json::to_string(&cmd)?;
        writeln!(stdin, "{}", json_req)?;
        stdin.flush()?;

        // レスポンス受信
        let stdout = process.stdout.as_mut().ok_or(BridgeError::Unknown("Failed to get stdout".into()))?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        if line.is_empty() {
             return Err(BridgeError::WorkerCrash("Worker closed stdout (EOF). This usually means the Python process crashed. Please check stderr output above for details.".into()));
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

    fn resolve_target_json(&self, target: &TnkValue) -> Result<serde_json::Value, BridgeError> {
        match target {
            TnkValue::Handle { id, .. } => Ok(serde_json::Value::String(id.clone())),
            TnkValue::Module { module } => Ok(serde_json::json!({
                "kind": "module",
                "module": module
            })),
            _ => Err(BridgeError::TypeMismatch(format!("Target object is not a handle or module. Got: {:?}", target))),
        }
    }

    pub fn call_function(
        &self, 
        target: &str, 
        args: &[&TnkValue],
        kwargs: Option<&std::collections::HashMap<String, &TnkValue>>,
    ) -> Result<TnkValue, BridgeError> {
        let cmd = BridgeCmd::CallFunction {
            session_id: self.session_id.clone(),
            req_id: Some(uuid::Uuid::new_v4().to_string()),
            target: target.to_string(),
            args: args.to_vec(),
            kwargs: kwargs.cloned(),
        };
        self.send_command(cmd)
    }

    pub fn call_method(
        &self, 
        target: &TnkValue, 
        method: &str, 
        args: &[&TnkValue],
        kwargs: Option<&std::collections::HashMap<String, &TnkValue>>,
    ) -> Result<TnkValue, BridgeError> {
        // TnkValue::Handle contains ID. If value, we might need to wrap?
        // Spec says target is ID string usually. 
        // But protocol.rs takes target: Value. 
        // We need to extract ID from Handle or send Value?
        // Existing implementation calls `resolve_target_json`.
        let target_json = self.resolve_target_json(target)?;
        
        let cmd = BridgeCmd::CallMethod {
            session_id: self.session_id.clone(),
            req_id: Some(uuid::Uuid::new_v4().to_string()),
            target: target_json,
            method: method.to_string(),
            args: args.to_vec(),
            kwargs: kwargs.cloned(),
        };
        self.send_command(cmd)
    }
    
    pub fn get_attribute(&self, target: &TnkValue, attribute: &str) -> Result<TnkValue, BridgeError> {
        let target_json = self.resolve_target_json(target)?;
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::GetAttribute {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_json,
            name: attribute.to_string(), // Changed 'name' to 'attribute' in the parameter, but the field in BridgeCmd is still 'name'.
        })
    }

    pub fn get_item(&self, target: &TnkValue, key: &TnkValue) -> Result<TnkValue, BridgeError> {
        let target_json = self.resolve_target_json(target)?;
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        self.send_command(BridgeCmd::GetItem {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_json,
            key: key.clone(),
        })
    }
    
    pub fn slice(&self, target: &TnkValue, start: Option<TnkValue>, stop: Option<TnkValue>, step: Option<TnkValue>) -> Result<TnkValue, BridgeError> {
        let target_json = self.resolve_target_json(target)?;
        let req_id = self.request_id.fetch_add(1, Ordering::SeqCst).to_string();
        
        // Convert Option<TnkValue> to TnkValue (None -> TnkValue::Value { value: None })
        let none_val = || TnkValue::Value { value: None };
        
        self.send_command(BridgeCmd::Slice {
            session_id: self.session_id.clone(),
            req_id: Some(req_id),
            target: target_json,
            start: start.unwrap_or_else(none_val),
            stop: stop.unwrap_or_else(none_val),
            step: step.unwrap_or_else(none_val),
        })
    }

    pub fn shutdown(&self) -> Result<(), BridgeError> {
        self.process.borrow_mut().kill().map_err(|e| BridgeError::Io(e))?;
        Ok(())
    }

    // --- V1.7.0 Module & Handle Refs ---

    /// Import a module and register it in the ModuleTable
    pub fn import(&self, py_name: &str, alias: &str) {
        self.modules.borrow_mut().insert(alias.to_string(), py_name.to_string());
    }

    /// bit.ly/TsuchinokoRefs
    pub fn get<'a>(&'a self, alias: &'a str) -> ModuleRef<'a> {
        ModuleRef {
            bridge: self,
            alias,
        }
    }
    
    pub fn handle<'a>(&'a self, id: String) -> HandleRef<'a> {
        HandleRef {
            bridge: self,
            handle_id: id,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ModuleRef<'a> {
    pub bridge: &'a PythonBridge,
    pub alias: &'a str,
}

impl<'a> ModuleRef<'a> {
    pub fn call_method(&self, method: &str, args: &[&TnkValue], kwargs: Option<&std::collections::HashMap<String, &TnkValue>>) -> Result<TnkValue, BridgeError> {
        // Resolve alias to module name
        let module_name = {
            let modules = self.bridge.modules.borrow();
            modules.get(self.alias).ok_or_else(|| BridgeError::Unknown(format!("Module alias '{}' not found in ModuleTable", self.alias)))?.clone()
        };
        
        let target_def = TnkValue::Module { module: module_name };
        self.bridge.call_method(&target_def, method, args, kwargs)
    }

    pub fn call(&self, args: &[&TnkValue], kwargs: Option<&std::collections::HashMap<String, &TnkValue>>) -> Result<TnkValue, BridgeError> {
        let module_name = {
            let modules = self.bridge.modules.borrow();
            modules.get(self.alias).ok_or_else(|| BridgeError::Unknown(format!("Module alias '{}' not found", self.alias)))?.clone()
        };
        self.bridge.call_function(&module_name, args, kwargs)
    }
    
    // Modules also support get_attribute (e.g. cv.some_constant)
    pub fn get_attribute(&self, attribute: &str) -> Result<TnkValue, BridgeError> {
        let module_name = {
            let modules = self.bridge.modules.borrow();
            modules.get(self.alias).ok_or_else(|| BridgeError::Unknown(format!("Module alias '{}' not found", self.alias)))?.clone()
        };
        let target_def = TnkValue::Module { module: module_name };
        self.bridge.get_attribute(&target_def, attribute)
    }
}

impl<'a> From<ModuleRef<'a>> for TnkValue {
    fn from(m: ModuleRef<'a>) -> Self {
        let modules = m.bridge.modules.borrow();
        let name = modules.get(m.alias).expect("Module alias not found in From conversion").clone();
        TnkValue::Module { module: name }
    }
}

#[derive(Clone)]
pub struct HandleRef<'a> {
    pub bridge: &'a PythonBridge,
    pub handle_id: String,
}

impl<'a> HandleRef<'a> {
    pub fn call_method(&self, method: &str, args: &[&TnkValue], kwargs: Option<&std::collections::HashMap<String, &TnkValue>>) -> Result<TnkValue, BridgeError> {
        // Construct a temporary TnkValue::Handle to pass to bridge.call_method
        // Ideally call_method should take ID string, but currently takes &TnkValue.
        // We can create a lightweight TnkValue::Handle wrapper.
        // Or refactor proper. For now, wrap.
        let target = TnkValue::Handle { 
            id: self.handle_id.clone(), 
            type_: "ref".to_string(), 
            repr: "ref".to_string(), 
            str_: "ref".to_string(),
            session_id: self.bridge.session_id.clone() 
        };
        self.bridge.call_method(&target, method, args, kwargs)
    }

    pub fn call(&self, args: &[&TnkValue], kwargs: Option<&std::collections::HashMap<String, &TnkValue>>) -> Result<TnkValue, BridgeError> {
        self.call_method("__call__", args, kwargs)
    }
    
    pub fn get_attribute(&self, attribute: &str) -> Result<TnkValue, BridgeError> {
        let target = TnkValue::Handle { 
            id: self.handle_id.clone(), 
            type_: "ref".to_string(), 
            repr: "ref".to_string(), 
            str_: "ref".to_string(),
            session_id: self.bridge.session_id.clone() 
        };
        self.bridge.get_attribute(&target, attribute)
    }
    
    pub fn get_item(&self, key: &TnkValue) -> Result<TnkValue, BridgeError> {
        let target = TnkValue::Handle { 
            id: self.handle_id.clone(), 
            type_: "ref".to_string(), 
            repr: "ref".to_string(), 
            str_: "ref".to_string(),
            session_id: self.bridge.session_id.clone() 
        };
        self.bridge.get_item(&target, key)
    }
}

impl<'a> From<HandleRef<'a>> for TnkValue {
    fn from(h: HandleRef<'a>) -> Self {
        TnkValue::Handle { 
            id: h.handle_id.clone(), 
            type_: "ref".to_string(), 
            repr: "ref".to_string(), 
            str_: "ref".to_string(),
            session_id: h.bridge.session_id.clone() 
        }
    }
}

pub fn display_value(value: &TnkValue) -> String {
    value.to_string()
}

// Compatibility layer for Emitter's generic calls
impl PythonBridge {
    pub fn call_json<T: serde::de::DeserializeOwned>(&mut self, target: &str, args: &[serde_json::Value]) -> Result<T, BridgeError> {
        let tnk_args: Vec<TnkValue> = args.iter().map(|v| crate::bridge::type_inference::from_value(v.clone())).collect();
        // call_function takes Vec<&TnkValue> (Zero-Copy Refactor)
        let args_refs: Vec<&TnkValue> = tnk_args.iter().collect();
        let result = self.call_function(target, &args_refs, None)?;
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
        // call_method takes &[&TnkValue] (Zero-Copy Refactor)
        let args_refs: Vec<&TnkValue> = tnk_args.iter().collect();
        let result = self.call_method(&target, method, &args_refs, None)?;
        let json_val: serde_json::Value = result.into();
        serde_json::from_value(json_val).map_err(BridgeError::Json)
    }
}

impl Drop for PythonBridge {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
