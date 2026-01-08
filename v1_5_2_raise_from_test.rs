
/// Python例外を表現するエラー型
#[derive(Debug, Clone)]
pub struct TsuchinokoError {
    /// 例外の種類 ("ValueError", "RuntimeError" 等)
    pub kind: String,
    /// エラーメッセージ
    pub message: String,
    /// 原因となった例外（raise from 用）
    pub cause: Option<Box<TsuchinokoError>>,
    /// ソースコード行番号（0 = 不明）
    pub line: usize,
}

impl TsuchinokoError {
    /// 新しいエラーを作成
    pub fn new(kind: &str, message: &str, cause: Option<TsuchinokoError>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.to_string(),
            cause: cause.map(Box::new),
            line: 0,
        }
    }
    
    /// 行番号付きでエラーを作成
    pub fn with_line(kind: &str, message: &str, line: usize, cause: Option<TsuchinokoError>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.to_string(),
            cause: cause.map(Box::new),
            line,
        }
    }
    
    /// 内部エラー（panic回収用）を作成
    pub fn internal(message: &str) -> Self {
        Self::new("InternalError", message, None)
    }
}

impl std::fmt::Display for TsuchinokoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 行番号があれば表示
        if self.line > 0 {
            write!(f, "[line {}] ", self.line)?;
        }
        write!(f, "{}: {}", self.kind, self.message)?;
        if let Some(cause) = &self.cause {
            write!(f, "\n  Caused by: {}", cause)?;
        }
        Ok(())
    }
}

impl std::error::Error for TsuchinokoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_ref().map(|c| c.as_ref() as &(dyn std::error::Error + 'static))
    }
}

fn validate_input(value: i64) -> Result<i64, TsuchinokoError> {
    // ""Validate input value with exception chaining""
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if value < 0i64 {
            panic!("[ValueError] {}", "negative value not allowed");
        }
    })) {
        Ok(__val) => __val,
        Err(__exc) => {
            let e = TsuchinokoError::new("Exception", &format!("{:?}", __exc), None);
            return Err(TsuchinokoError::with_line("RuntimeError", &format!("{}", "validation failed"), 10, Some(e)));
        }
    }

    return Ok((value * 2i64));
}
fn main() {
    let result = std::panic::catch_unwind(|| {
    let result: i64 = validate_input(5i64).unwrap();
    println!("{:?}", &result);

    });
    if let Err(e) = result {
        let msg = if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        eprintln!("InternalError: {}", msg);
        std::process::exit(1);
    }
}