use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::io::{self, BufRead, Write};

// --- Data Types (Tagged Union) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TnkValue {
    Value {
        value: Option<JsonPrimitive>, // number | string | boolean | null
    },
    Handle {
        id: String,
        #[serde(rename = "type")]
        type_: String,
        repr: String,
        #[serde(rename = "str")]
        str_: String,
        session_id: String,
    },
    List {
        items: Vec<TnkValue>,
    },
    Tuple {
        items: Vec<TnkValue>,
    },
    Dict {
        items: Vec<DictItem>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonPrimitive {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictItem {
    key: TnkValue,
    value: TnkValue,
}

// --- Commands ---

#[derive(Debug, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    CallMethod {
        session_id: String,
        req_id: Option<String>,
        target: String,
        method: String,
        args: Vec<TnkValue>,
    },
    GetAttribute {
        session_id: String,
        req_id: Option<String>,
        target: String,
        name: String,
    },
    GetItem {
        session_id: String,
        req_id: Option<String>,
        target: String,
        key: TnkValue,
    },
    Slice {
        session_id: String,
        req_id: Option<String>,
        target: String,
        start: TnkValue,
        stop: TnkValue,
        step: TnkValue,
    },
    Iter {
        session_id: String,
        req_id: Option<String>,
        target: String,
    },
    IterNextBatch {
        session_id: String,
        req_id: Option<String>,
        target: String,
        batch_size: usize,
    },
    Delete {
        session_id: String,
        req_id: Option<String>,
        target: String,
    },
    
    // Debug Commands
    DebugCreateString {
        session_id: String,
        req_id: Option<String>,
        value: String,
    },
    DebugEval {
        session_id: String,
        req_id: Option<String>,
        code: String,
    },
}

// --- Response ---

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Response {
    Ok {
        req_id: Option<String>,
        value: TnkValue,
        meta: Option<ResponseMeta>,
    },
    Error {
        req_id: Option<String>,
        error: BridgeErrorDetail,
    },
}

#[derive(Debug, Deserialize)]
pub struct ResponseMeta {
    pub done: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BridgeErrorDetail {
    pub code: String,
    pub py_type: Option<String>,
    pub message: String,
    pub traceback: Option<String>,
    pub op: Option<Value>,
}

fn main() -> io::Result<()> {
    // Basic scenario to verify protocol
    let session_id = "sess-proto-1".to_string();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = stdin.lock();

    // Helper to send command
    let mut send = |cmd: Command| -> io::Result<Response> {
        let json = serde_json::to_string(&cmd)?;
        writeln!(stdout, "{}", json)?;
        stdout.flush()?;

        let mut line = String::new();
        reader.read_line(&mut line)?;
        let resp: Response = serde_json::from_str(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        eprintln!("Rust received: {:?}", resp); // Log to stderr
        Ok(resp)
    };

    eprintln!("--- Bridge Proto Start ---");

    // 1. Create String Handle
    eprintln!("1. Creating String Handle...");
    let resp = send(Command::DebugCreateString {
        session_id: session_id.clone(),
        req_id: Some("req-1".to_string()),
        value: "Python from Rust".to_string(),
    })?;

    let h_id = match resp {
        Response::Ok { value: TnkValue::Handle { id, .. }, .. } => id,
        _ => panic!("Expected handle"),
    };
    eprintln!("Obtained Handle: {}", h_id);

    // 2. Call Method .upper()
    eprintln!("2. Calling .upper()...");
    let resp = send(Command::CallMethod {
        session_id: session_id.clone(),
        req_id: Some("req-2".to_string()),
        target: h_id.clone(),
        method: "upper".to_string(),
        args: vec![],
    })?;
    
    match resp {
        Response::Ok { value: TnkValue::Value { value: Some(JsonPrimitive::String(s)) }, .. } => {
            eprintln!("Result: {}", s);
            assert_eq!(s, "PYTHON FROM RUST");
        },
        _ => panic!("Expected string value"),
    }

    // 3. Create List for Iteration
    eprintln!("3. Creating List [1, 2, 3]...");
    let resp = send(Command::DebugEval {
        session_id: session_id.clone(),
        req_id: Some("req-3".to_string()),
        code: "[1, 2, 3]".to_string(),
    })?;
    let list_id = match resp {
        Response::Ok { value: TnkValue::Handle { id, .. }, .. } => id,
        _ => panic!("Expected list handle"),
    };

    // 4. Get Iterator
    eprintln!("4. Getting Iterator...");
    let resp = send(Command::Iter {
        session_id: session_id.clone(),
        req_id: Some("req-4".to_string()),
        target: list_id,
    })?;
    let iter_id = match resp {
        Response::Ok { value: TnkValue::Handle { id, .. }, .. } => id,
        _ => panic!("Expected iterator handle"),
    };

    // 5. Next Batch
    eprintln!("5. Fetching batch...");
    let resp = send(Command::IterNextBatch {
        session_id: session_id.clone(),
        req_id: Some("req-5".to_string()),
        target: iter_id,
        batch_size: 2,
    })?;
    
    match resp {
        Response::Ok { value: TnkValue::List { items }, meta, .. } => {
            eprintln!("Batch items: {:?}", items);
            assert_eq!(items.len(), 2);
            if let Some(m) = meta {
                eprintln!("Done: {:?}", m.done);
            }
        },
        _ => panic!("Expected list response"),
    }

    eprintln!("--- Bridge Proto Test Complete ---");
    Ok(())
}
