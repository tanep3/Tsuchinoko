use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Protocol Error: {0}")]
    Protocol(String),

    #[error("Stale Handle: {0} (Session ID mismatch or expired)")]
    StaleHandle(String),

    #[error("Worker Crash: {0}")]
    WorkerCrash(String),

    #[error("Value Too Large: {0}")]
    ValueTooLarge(String),

    #[error("Security Violation: {0}")]
    Security(String),

    #[error("Python Exception ({py_type}): {message}")]
    PythonException {
        py_type: String,
        message: String,
        traceback: Option<String>,
    },

    #[error("Type Mismatch: {0}")]
    TypeMismatch(String),

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization Error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unknown Error: {0}")]
    Unknown(String),
}

impl BridgeError {
    /// Convert from API error code/message to BridgeError
    pub fn from_api_error(
        code: &str,
        message: String,
        py_type: Option<String>,
        traceback: Option<String>,
    ) -> Self {
        match code {
            "ProtocolError" => BridgeError::Protocol(message),
            "StaleHandle" => BridgeError::StaleHandle(message),
            "WorkerCrash" => BridgeError::WorkerCrash(message),
            "ValueTooLarge" => BridgeError::ValueTooLarge(message),
            "SecurityViolation" => BridgeError::Security(message),
            "PythonException" => BridgeError::PythonException {
                py_type: py_type.unwrap_or_else(|| "Exception".to_string()),
                message,
                traceback,
            },
            "TypeMismatch" => BridgeError::TypeMismatch(message),
            _ => BridgeError::Unknown(format!("{}: {}", code, message)),
        }
    }
}
