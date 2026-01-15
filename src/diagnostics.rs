//! TnkDiagnostics - compile-time diagnostics collection and output

use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticSpan {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct TnkDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub span: DiagnosticSpan,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TnkDiagnostics {
    pub diagnostics: Vec<TnkDiagnostic>,
}

impl TnkDiagnostics {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn add(&mut self, diag: TnkDiagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn extend(&mut self, other: TnkDiagnostics) {
        self.diagnostics.extend(other.diagnostics);
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for diag in &self.diagnostics {
            let file = diag.span.file.as_deref().unwrap_or("<input>");
            let line = diag.span.line;
            let col = diag.span.column;
            out.push_str(&format!(
                "[{}] {}:{}:{} {}\n",
                diag.code, file, line, col, diag.message
            ));
        }
        out
    }
}

pub fn span_for_line(
    file: Option<&Path>,
    line: usize,
    column: usize,
    len: usize,
) -> DiagnosticSpan {
    let end_col = if len == 0 {
        column
    } else {
        column + len.saturating_sub(1)
    };
    DiagnosticSpan {
        file: file.map(|p| p.display().to_string()),
        line,
        column,
        end_line: line,
        end_column: end_col,
    }
}

pub fn error_diag(code: &str, message: String, span: DiagnosticSpan, phase: &str) -> TnkDiagnostic {
    TnkDiagnostic {
        code: code.to_string(),
        message,
        severity: DiagnosticSeverity::Error,
        span,
        phase: phase.to_string(),
        meta: None,
    }
}

pub fn from_error(err: &crate::error::TsuchinokoError, file: Option<&Path>) -> TnkDiagnostics {
    let mut diags = TnkDiagnostics::new();
    let (code, message, line, phase) = match err {
        crate::error::TsuchinokoError::ParseError { line, message } => {
            ("TNK-PARSE-ERROR", message.clone(), *line, "parse")
        }
        crate::error::TsuchinokoError::TypeError { line, message } => {
            ("TNK-TYPE-ERROR", message.clone(), *line, "semantic")
        }
        crate::error::TsuchinokoError::UndefinedVariable { name, line } => (
            "TNK-UNDEFINED-VARIABLE",
            format!("Undefined variable '{name}'"),
            *line,
            "semantic",
        ),
        crate::error::TsuchinokoError::UnsupportedSyntax { syntax, line } => (
            "TNK-UNSUPPORTED-SYNTAX",
            format!("Unsupported syntax: {syntax}"),
            *line,
            "semantic",
        ),
        crate::error::TsuchinokoError::SemanticError { message } => {
            ("TNK-SEMANTIC-ERROR", message.clone(), 1, "semantic")
        }
        crate::error::TsuchinokoError::CompileError(message) => {
            ("TNK-COMPILE-ERROR", message.clone(), 1, "lowering")
        }
        crate::error::TsuchinokoError::IoError(_) => ("TNK-IO-ERROR", format!("{err}"), 1, "parse"),
    };
    let span = span_for_line(file, line, 1, 1);
    diags.add(error_diag(code, message, span, phase));
    diags
}

fn strip_trailing_comment(line: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut string_char = ' ';
    let mut escape = false;
    for c in line.chars() {
        if in_string {
            out.push(c);
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
            } else if c == string_char {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c;
                out.push(c);
            }
            '#' => break,
            _ => out.push(c),
        }
    }
    out
}

fn find_keyword(line: &str, keyword: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let k = keyword.as_bytes();
    if k.is_empty() {
        return None;
    }
    let mut i = 0usize;
    while i + k.len() <= bytes.len() {
        if &bytes[i..i + k.len()] == k {
            let left_ok =
                i == 0 || !((bytes[i - 1] as char).is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            let right_ok = i + k.len() == bytes.len()
                || !((bytes[i + k.len()] as char).is_ascii_alphanumeric()
                    || bytes[i + k.len()] == b'_');
            if left_ok && right_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn find_builtin_call(line: &str, name: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let k = name.as_bytes();
    if k.is_empty() {
        return None;
    }
    let mut i = 0usize;
    while i + k.len() <= bytes.len() {
        if &bytes[i..i + k.len()] == k {
            let left_ok =
                i == 0 || !((bytes[i - 1] as char).is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            let right_ok = i + k.len() == bytes.len()
                || !((bytes[i + k.len()] as char).is_ascii_alphanumeric()
                    || bytes[i + k.len()] == b'_');
            if left_ok && right_ok {
                if let Some(def_pos) = line.find("def ") {
                    if i == def_pos + 4 {
                        i += 1;
                        continue;
                    }
                }
                if i > 0 && bytes[i - 1] == b'.' {
                    i += 1;
                    continue;
                }
                let mut j = i + k.len();
                while j < bytes.len() && (bytes[j] as char).is_ascii_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'(' {
                    return Some(i);
                }
            }
        }
        i += 1;
    }
    None
}

pub fn scan_unsupported_syntax(
    source: &str,
    file: Option<&Path>,
    registry: &crate::unsupported_features::UnsupportedFeatureRegistry,
) -> TnkDiagnostics {
    use crate::unsupported_features::UnsupportedFeature as UF;
    let mut diags = TnkDiagnostics::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_trailing_comment(raw_line);
        let line = mask_string_literals(&line);
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with('@') {
            let col = line.find('@').unwrap_or(0) + 1;
            let decorator = trimmed.strip_prefix('@').unwrap_or("").trim();
            let is_allowed = decorator == "dataclass"
                || decorator == "staticmethod"
                || decorator == "property"
                || decorator.ends_with(".setter");

            if !is_allowed {
                if decorator == "classmethod" && registry.is_enabled(UF::ClassMethodDecorator) {
                    let span = span_for_line(file, line_no, col, "@classmethod".len());
                    diags.add(error_diag(
                        "TNK-UNSUPPORTED-SYNTAX",
                        "unsupported decorator: @classmethod".to_string(),
                        span,
                        "parse",
                    ));
                } else if registry.is_enabled(UF::CustomDecorator) {
                    let span = span_for_line(file, line_no, col, decorator.len() + 1);
                    diags.add(error_diag(
                        "TNK-UNSUPPORTED-SYNTAX",
                        format!("unsupported decorator: @{decorator}"),
                        span,
                        "parse",
                    ));
                }
            }
        }

        let checks: [(&str, UF, &str); 22] = [
            (
                "match",
                UF::MatchStatement,
                "match statement is unsupported",
            ),
            ("async", UF::AsyncDef, "async is unsupported"),
            ("await", UF::AwaitExpr, "await is unsupported"),
            ("yield from", UF::YieldFrom, "yield from is unsupported"),
            ("yield", UF::YieldStatement, "yield is unsupported"),
            ("del", UF::DelStatement, "del statement is unsupported"),
            (
                "global",
                UF::GlobalStatement,
                "global statement is unsupported",
            ),
            (
                "nonlocal",
                UF::NonlocalStatement,
                "nonlocal statement is unsupported",
            ),
            (":=", UF::WalrusOperator, "walrus operator is unsupported"),
            ("async for", UF::AsyncFor, "async for is unsupported"),
            ("async with", UF::AsyncWith, "async with is unsupported"),
            (
                "def __iter__",
                UF::MagicMethodIter,
                "unsupported magic method: __iter__",
            ),
            (
                "def __next__",
                UF::MagicMethodNext,
                "unsupported magic method: __next__",
            ),
            (
                "def __slots__",
                UF::MagicMethodSlots,
                "unsupported magic method: __slots__",
            ),
            (
                "def __call__",
                UF::MagicMethodCall,
                "unsupported magic method: __call__",
            ),
            (
                "def __repr__",
                UF::MagicMethodRepr,
                "unsupported magic method: __repr__",
            ),
            (
                "def __str__",
                UF::MagicMethodStr,
                "unsupported magic method: __str__",
            ),
            (
                "def __getitem__",
                UF::MagicMethodGetItem,
                "unsupported magic method: __getitem__",
            ),
            (
                "def __setitem__",
                UF::MagicMethodSetItem,
                "unsupported magic method: __setitem__",
            ),
            (
                "def __delitem__",
                UF::MagicMethodDelItem,
                "unsupported magic method: __delitem__",
            ),
            (
                "def __len__",
                UF::MagicMethodLen,
                "unsupported magic method: __len__",
            ),
            (
                "def __contains__",
                UF::MagicMethodContains,
                "unsupported magic method: __contains__",
            ),
        ];

        for (kw, feat, msg) in checks {
            if !registry.is_enabled(feat) {
                continue;
            }
            let pos = if kw.contains(' ') || kw == ":=" {
                line.find(kw)
            } else {
                find_keyword(&line, kw)
            };
            if let Some(col) = pos {
                let span = span_for_line(file, line_no, col + 1, kw.len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    msg.to_string(),
                    span,
                    "parse",
                ));
            }
        }

        if registry.is_enabled(UF::TypeStatement) {
            let trimmed = line.trim_start();
            if trimmed.starts_with("type ") && trimmed.contains('=') {
                let col = line.find('t').unwrap_or(0) + 1;
                let span = span_for_line(file, line_no, col, "type".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "type statement is unsupported".to_string(),
                    span,
                    "parse",
                ));
            }
        }

        if registry.is_enabled(UF::BuiltinIter) {
            if let Some(col) = find_builtin_call(&line, "iter") {
                let span = span_for_line(file, line_no, col + 1, "iter".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: iter()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinNext) {
            if let Some(col) = find_builtin_call(&line, "next") {
                let span = span_for_line(file, line_no, col + 1, "next".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: next()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinGetattr) {
            if let Some(col) = find_builtin_call(&line, "getattr") {
                let span = span_for_line(file, line_no, col + 1, "getattr".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: getattr()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinSetattr) {
            if let Some(col) = find_builtin_call(&line, "setattr") {
                let span = span_for_line(file, line_no, col + 1, "setattr".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: setattr()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinHasattr) {
            if let Some(col) = find_builtin_call(&line, "hasattr") {
                let span = span_for_line(file, line_no, col + 1, "hasattr".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: hasattr()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinDelattr) {
            if let Some(col) = find_builtin_call(&line, "delattr") {
                let span = span_for_line(file, line_no, col + 1, "delattr".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: delattr()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinDir) {
            if let Some(col) = find_builtin_call(&line, "dir") {
                let span = span_for_line(file, line_no, col + 1, "dir".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: dir()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinVars) {
            if let Some(col) = find_builtin_call(&line, "vars") {
                let span = span_for_line(file, line_no, col + 1, "vars".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: vars()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinType) {
            if let Some(col) = find_builtin_call(&line, "type") {
                let span = span_for_line(file, line_no, col + 1, "type".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: type()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinIssubclass) {
            if let Some(col) = find_builtin_call(&line, "issubclass") {
                let span = span_for_line(file, line_no, col + 1, "issubclass".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: issubclass()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinId) {
            if let Some(col) = find_builtin_call(&line, "id") {
                let span = span_for_line(file, line_no, col + 1, "id".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: id()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinHash) {
            if let Some(col) = find_builtin_call(&line, "hash") {
                let span = span_for_line(file, line_no, col + 1, "hash".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: hash()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinFormat) {
            if let Some(col) = find_builtin_call(&line, "format") {
                let span = span_for_line(file, line_no, col + 1, "format".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: format()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinRepr) {
            if let Some(col) = find_builtin_call(&line, "repr") {
                let span = span_for_line(file, line_no, col + 1, "repr".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: repr()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinObject) {
            if let Some(col) = find_builtin_call(&line, "object") {
                let span = span_for_line(file, line_no, col + 1, "object".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: object()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinCompile) {
            if let Some(col) = find_builtin_call(&line, "compile") {
                let span = span_for_line(file, line_no, col + 1, "compile".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: compile()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinMemoryview) {
            if let Some(col) = find_builtin_call(&line, "memoryview") {
                let span = span_for_line(file, line_no, col + 1, "memoryview".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: memoryview()".to_string(),
                    span,
                    "parse",
                ));
            }
        }
        if registry.is_enabled(UF::BuiltinBytearray) {
            if let Some(col) = find_builtin_call(&line, "bytearray") {
                let span = span_for_line(file, line_no, col + 1, "bytearray".len());
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    "unsupported builtin: bytearray()".to_string(),
                    span,
                    "parse",
                ));
            }
        }

        if registry.is_enabled(UF::MultipleInheritance) && line.trim_start().starts_with("class ") {
            if let Some(paren_start) = line.find('(') {
                if let Some(paren_end) = line[paren_start + 1..].find(')') {
                    let inside = &line[paren_start + 1..paren_start + 1 + paren_end];
                    if inside.split(',').filter(|s| !s.trim().is_empty()).count() > 1 {
                        let span = span_for_line(file, line_no, paren_start + 1, 5);
                        diags.add(error_diag(
                            "TNK-UNSUPPORTED-SYNTAX",
                            "multiple inheritance is unsupported".to_string(),
                            span,
                            "parse",
                        ));
                    }
                }
            }
        }
    }
    diags
}

fn mask_string_literals(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut string_char = ' ';
    let mut escape = false;
    for c in line.chars() {
        if in_string {
            // Preserve string length for column accuracy.
            out.push(' ');
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
            } else if c == string_char {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c;
                out.push(' ');
            }
            _ => out.push(c),
        }
    }
    out
}

pub fn scan_unsupported_ast(
    program: &crate::parser::Program,
    file: Option<&Path>,
    registry: &crate::unsupported_features::UnsupportedFeatureRegistry,
) -> TnkDiagnostics {
    use crate::parser::{Expr, Stmt};
    use crate::unsupported_features::UnsupportedFeature as UF;

    let mut diags = TnkDiagnostics::new();
    let span = span_for_line(file, 1, 1, 1);

    #[allow(clippy::only_used_in_recursion)]
    fn scan_expr(
        expr: &Expr,
        diags: &mut TnkDiagnostics,
        span: &DiagnosticSpan,
        registry: &crate::unsupported_features::UnsupportedFeatureRegistry,
    ) {
        match expr {
            Expr::GenExpr { .. } => {}
            Expr::BinOp { left, right, .. } => {
                scan_expr(left, diags, span, registry);
                scan_expr(right, diags, span, registry);
            }
            Expr::UnaryOp { operand, .. } => scan_expr(operand, diags, span, registry),
            Expr::Call { func, args, kwargs } => {
                scan_expr(func, diags, span, registry);
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
                for (_, arg) in kwargs {
                    scan_expr(arg, diags, span, registry);
                }
            }
            Expr::List(items) | Expr::Tuple(items) | Expr::Set(items) => {
                for item in items {
                    scan_expr(item, diags, span, registry);
                }
            }
            Expr::Dict(items) => {
                for (k, v) in items {
                    scan_expr(k, diags, span, registry);
                    scan_expr(v, diags, span, registry);
                }
            }
            Expr::ListComp {
                elt,
                iter,
                condition,
                ..
            }
            | Expr::SetComp {
                elt,
                iter,
                condition,
                ..
            } => {
                scan_expr(elt, diags, span, registry);
                scan_expr(iter, diags, span, registry);
                if let Some(cond) = condition {
                    scan_expr(cond, diags, span, registry);
                }
            }
            Expr::DictComp {
                key,
                value,
                iter,
                condition,
                ..
            } => {
                scan_expr(key, diags, span, registry);
                scan_expr(value, diags, span, registry);
                scan_expr(iter, diags, span, registry);
                if let Some(cond) = condition {
                    scan_expr(cond, diags, span, registry);
                }
            }
            Expr::IfExp { test, body, orelse } => {
                scan_expr(test, diags, span, registry);
                scan_expr(body, diags, span, registry);
                scan_expr(orelse, diags, span, registry);
            }
            Expr::Index { target, index } => {
                scan_expr(target, diags, span, registry);
                scan_expr(index, diags, span, registry);
            }
            Expr::Slice {
                target,
                start,
                end,
                step,
            } => {
                scan_expr(target, diags, span, registry);
                if let Some(v) = start {
                    scan_expr(v, diags, span, registry);
                }
                if let Some(v) = end {
                    scan_expr(v, diags, span, registry);
                }
                if let Some(v) = step {
                    scan_expr(v, diags, span, registry);
                }
            }
            Expr::Attribute { value, .. } => scan_expr(value, diags, span, registry),
            Expr::FString { values, .. } => {
                for v in values {
                    scan_expr(v, diags, span, registry);
                }
            }
            Expr::Lambda { body, .. } => scan_expr(body, diags, span, registry),
            Expr::Starred(inner) => scan_expr(inner, diags, span, registry),
            Expr::IntLiteral(_)
            | Expr::FloatLiteral(_)
            | Expr::StringLiteral(_)
            | Expr::BoolLiteral(_)
            | Expr::NoneLiteral
            | Expr::Ident(_) => {}
        }
    }

    fn is_open_call(expr: &Expr) -> bool {
        if let Expr::Call { func, .. } = expr {
            if let Expr::Ident(name) = func.as_ref() {
                return name == "open";
            }
        }
        false
    }

    fn scan_stmt(
        stmt: &Stmt,
        diags: &mut TnkDiagnostics,
        span: &DiagnosticSpan,
        registry: &crate::unsupported_features::UnsupportedFeatureRegistry,
    ) {
        match stmt {
            Stmt::Assign { value, .. } => scan_expr(value, diags, span, registry),
            Stmt::IndexAssign {
                target,
                index,
                value,
            } => {
                scan_expr(target, diags, span, registry);
                scan_expr(index, diags, span, registry);
                scan_expr(value, diags, span, registry);
            }
            Stmt::AugAssign { value, .. } => scan_expr(value, diags, span, registry),
            Stmt::TupleAssign { value, .. } => scan_expr(value, diags, span, registry),
            Stmt::IndexSwap {
                left_targets,
                right_values,
            } => {
                for expr in left_targets {
                    scan_expr(expr, diags, span, registry);
                }
                for expr in right_values {
                    scan_expr(expr, diags, span, registry);
                }
            }
            Stmt::FuncDef { body, .. } => {
                for s in body {
                    scan_stmt(s, diags, span, registry);
                }
            }
            Stmt::If {
                condition,
                then_body,
                elif_clauses,
                else_body,
            } => {
                scan_expr(condition, diags, span, registry);
                for s in then_body {
                    scan_stmt(s, diags, span, registry);
                }
                for (cond, body) in elif_clauses {
                    scan_expr(cond, diags, span, registry);
                    for s in body {
                        scan_stmt(s, diags, span, registry);
                    }
                }
                if let Some(body) = else_body {
                    for s in body {
                        scan_stmt(s, diags, span, registry);
                    }
                }
            }
            Stmt::For { iter, body, .. } => {
                scan_expr(iter, diags, span, registry);
                for s in body {
                    scan_stmt(s, diags, span, registry);
                }
            }
            Stmt::While { condition, body } => {
                scan_expr(condition, diags, span, registry);
                for s in body {
                    scan_stmt(s, diags, span, registry);
                }
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    scan_expr(expr, diags, span, registry);
                }
            }
            Stmt::Expr(expr) => scan_expr(expr, diags, span, registry),
            Stmt::ClassDef { bases, methods, .. } => {
                if bases.len() > 1 && registry.is_enabled(UF::MultipleInheritance) {
                    diags.add(error_diag(
                        "TNK-UNSUPPORTED-SYNTAX",
                        "multiple inheritance is unsupported".to_string(),
                        span.clone(),
                        "semantic",
                    ));
                }
                for method in methods {
                    for s in &method.body {
                        scan_stmt(s, diags, span, registry);
                    }
                }
            }
            Stmt::TryExcept {
                try_body,
                except_clauses,
                else_body,
                finally_body,
            } => {
                for s in try_body {
                    scan_stmt(s, diags, span, registry);
                }
                for c in except_clauses {
                    for s in &c.body {
                        scan_stmt(s, diags, span, registry);
                    }
                }
                if let Some(body) = else_body {
                    for s in body {
                        scan_stmt(s, diags, span, registry);
                    }
                }
                if let Some(body) = finally_body {
                    for s in body {
                        scan_stmt(s, diags, span, registry);
                    }
                }
            }
            Stmt::Raise { message, cause, .. } => {
                scan_expr(message, diags, span, registry);
                if let Some(expr) = cause {
                    scan_expr(expr, diags, span, registry);
                }
            }
            Stmt::Assert { test, msg } => {
                scan_expr(test, diags, span, registry);
                if let Some(expr) = msg {
                    scan_expr(expr, diags, span, registry);
                }
            }
            Stmt::With {
                context_expr, body, ..
            } => {
                if !is_open_call(context_expr) && registry.is_enabled(UF::CustomContextManager) {
                    diags.add(error_diag(
                        "TNK-UNSUPPORTED-SYNTAX",
                        "custom context manager is unsupported".to_string(),
                        span.clone(),
                        "semantic",
                    ));
                }
                scan_expr(context_expr, diags, span, registry);
                for s in body {
                    scan_stmt(s, diags, span, registry);
                }
            }
            Stmt::Import { .. } | Stmt::Break | Stmt::Continue => {}
        }
    }

    for stmt in &program.statements {
        scan_stmt(stmt, &mut diags, &span, registry);
    }

    diags
}

pub fn scan_unsupported_ir(
    nodes: &[crate::ir::IrNode],
    file: Option<&Path>,
    registry: &crate::unsupported_features::UnsupportedFeatureRegistry,
) -> TnkDiagnostics {
    use crate::ir::{IrExpr, IrExprKind, IrNode};
    use crate::unsupported_features::UnsupportedFeature as UF;

    let mut diags = TnkDiagnostics::new();
    let span = span_for_line(file, 1, 1, 1);

    #[allow(clippy::only_used_in_recursion)]
    fn scan_expr(
        expr: &IrExpr,
        diags: &mut TnkDiagnostics,
        span: &DiagnosticSpan,
        registry: &crate::unsupported_features::UnsupportedFeatureRegistry,
    ) {
        match &expr.kind {
            IrExprKind::ListComp {
                elt,
                iter,
                condition,
                ..
            }
            | IrExprKind::SetComp {
                elt,
                iter,
                condition,
                ..
            } => {
                scan_expr(elt, diags, span, registry);
                scan_expr(iter, diags, span, registry);
                if let Some(cond) = condition {
                    scan_expr(cond, diags, span, registry);
                }
            }
            IrExprKind::DictComp {
                key,
                value,
                iter,
                condition,
                ..
            } => {
                scan_expr(key, diags, span, registry);
                scan_expr(value, diags, span, registry);
                scan_expr(iter, diags, span, registry);
                if let Some(cond) = condition {
                    scan_expr(cond, diags, span, registry);
                }
            }
            IrExprKind::BinOp { left, right, .. } => {
                scan_expr(left, diags, span, registry);
                scan_expr(right, diags, span, registry);
            }
            IrExprKind::UnaryOp { operand, .. } => scan_expr(operand, diags, span, registry),
            IrExprKind::Call { func, args, .. } => {
                scan_expr(func, diags, span, registry);
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
            }
            IrExprKind::MethodCall { target, args, .. } => {
                scan_expr(target, diags, span, registry);
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
            }
            IrExprKind::PyO3MethodCall { target, args, .. } => {
                scan_expr(target, diags, span, registry);
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
            }
            IrExprKind::IfExp { test, body, orelse } => {
                scan_expr(test, diags, span, registry);
                scan_expr(body, diags, span, registry);
                scan_expr(orelse, diags, span, registry);
            }
            IrExprKind::BridgeMethodCall {
                target,
                args,
                keywords,
                ..
            } => {
                scan_expr(target, diags, span, registry);
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
                for (_, arg) in keywords {
                    scan_expr(arg, diags, span, registry);
                }
            }
            IrExprKind::BridgeCall {
                target,
                args,
                keywords,
            } => {
                scan_expr(target, diags, span, registry);
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
                for (_, arg) in keywords {
                    scan_expr(arg, diags, span, registry);
                }
            }
            IrExprKind::FieldAccess { target, .. }
            | IrExprKind::BridgeAttributeAccess { target, .. }
            | IrExprKind::BridgeItemAccess { target, .. }
            | IrExprKind::BridgeSlice { target, .. }
            | IrExprKind::Ref(target)
            | IrExprKind::TnkValueFrom(target)
            | IrExprKind::FromTnkValue { value: target, .. }
            | IrExprKind::Unwrap(target) => {
                scan_expr(target, diags, span, registry);
            }
            IrExprKind::Index { target, index } => {
                scan_expr(target, diags, span, registry);
                scan_expr(index, diags, span, registry);
            }
            IrExprKind::Slice {
                target,
                start,
                end,
                step,
            } => {
                scan_expr(target, diags, span, registry);
                if let Some(v) = start {
                    scan_expr(v, diags, span, registry);
                }
                if let Some(v) = end {
                    scan_expr(v, diags, span, registry);
                }
                if let Some(v) = step {
                    scan_expr(v, diags, span, registry);
                }
            }
            IrExprKind::StructConstruct { fields, .. } => {
                for (_, value) in fields {
                    scan_expr(value, diags, span, registry);
                }
            }
            IrExprKind::List { elements, .. }
            | IrExprKind::Set { elements, .. }
            | IrExprKind::Tuple(elements) => {
                for e in elements {
                    scan_expr(e, diags, span, registry);
                }
            }
            IrExprKind::Dict { entries, .. } => {
                for (k, v) in entries {
                    scan_expr(k, diags, span, registry);
                    scan_expr(v, diags, span, registry);
                }
            }
            IrExprKind::FString { values, .. } => {
                for (v, _) in values {
                    scan_expr(v, diags, span, registry);
                }
            }
            IrExprKind::Print { args } => {
                for (expr, _) in args {
                    scan_expr(expr, diags, span, registry);
                }
            }
            IrExprKind::Closure { body, .. } => {
                for node in body {
                    scan_node(node, diags, span, registry);
                }
            }
            IrExprKind::DynamicWrap { value, .. }
            | IrExprKind::Cast { target: value, .. }
            | IrExprKind::JsonConversion { target: value, .. }
            | IrExprKind::BoxNew(value)
            | IrExprKind::Reference { target: value }
            | IrExprKind::MutReference { target: value } => {
                scan_expr(value, diags, span, registry);
            }
            IrExprKind::BuiltinCall { args, .. } => {
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
            }
            IrExprKind::StaticCall { args, .. } => {
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
            }
            IrExprKind::PyO3Call { args, .. } => {
                for arg in args {
                    scan_expr(arg, diags, span, registry);
                }
            }
            IrExprKind::Sorted { iter, key, .. } => {
                scan_expr(iter, diags, span, registry);
                if let Some(k) = key {
                    scan_expr(k, diags, span, registry);
                }
            }
            IrExprKind::IntLit(_)
            | IrExprKind::FloatLit(_)
            | IrExprKind::StringLit(_)
            | IrExprKind::BoolLit(_)
            | IrExprKind::NoneLit
            | IrExprKind::Var(_)
            | IrExprKind::ConstRef { .. }
            | IrExprKind::RawCode(_)
            | IrExprKind::BridgeGet { .. }
            | IrExprKind::Range { .. } => {}
        }
    }

    fn scan_magic_method(
        name: &str,
        diags: &mut TnkDiagnostics,
        span: &DiagnosticSpan,
        registry: &crate::unsupported_features::UnsupportedFeatureRegistry,
    ) {
        let feature = match name {
            "__iter__" => Some(UF::MagicMethodIter),
            "__next__" => Some(UF::MagicMethodNext),
            "__slots__" => Some(UF::MagicMethodSlots),
            "__call__" => Some(UF::MagicMethodCall),
            "__repr__" => Some(UF::MagicMethodRepr),
            "__str__" => Some(UF::MagicMethodStr),
            "__getitem__" => Some(UF::MagicMethodGetItem),
            "__setitem__" => Some(UF::MagicMethodSetItem),
            "__delitem__" => Some(UF::MagicMethodDelItem),
            "__len__" => Some(UF::MagicMethodLen),
            "__contains__" => Some(UF::MagicMethodContains),
            _ => None,
        };
        if let Some(feature) = feature {
            if registry.is_enabled(feature) {
                diags.add(error_diag(
                    "TNK-UNSUPPORTED-SYNTAX",
                    format!("unsupported magic method: {name}"),
                    span.clone(),
                    "lowering",
                ));
            }
        }
    }

    fn scan_node(
        node: &IrNode,
        diags: &mut TnkDiagnostics,
        span: &DiagnosticSpan,
        registry: &crate::unsupported_features::UnsupportedFeatureRegistry,
    ) {
        match node {
            IrNode::Match { .. } => {
                if registry.is_enabled(UF::MatchStatement) {
                    diags.add(error_diag(
                        "TNK-UNSUPPORTED-SYNTAX",
                        "match statement is unsupported".to_string(),
                        span.clone(),
                        "lowering",
                    ));
                }
            }
            IrNode::VarDecl { init, .. } => {
                if let Some(expr) = init {
                    scan_expr(expr, diags, span, registry);
                }
            }
            IrNode::Assign { value, .. }
            | IrNode::AugAssign { value, .. }
            | IrNode::MultiAssign { value, .. }
            | IrNode::FieldAssign { value, .. } => {
                scan_expr(value, diags, span, registry);
            }
            IrNode::IndexAssign {
                target,
                index,
                value,
            } => {
                scan_expr(target, diags, span, registry);
                scan_expr(index, diags, span, registry);
                scan_expr(value, diags, span, registry);
            }
            IrNode::MultiVarDecl { value, .. } => scan_expr(value, diags, span, registry),
            IrNode::If {
                cond,
                then_block,
                else_block,
            } => {
                scan_expr(cond, diags, span, registry);
                for node in then_block {
                    scan_node(node, diags, span, registry);
                }
                if let Some(nodes) = else_block {
                    for node in nodes {
                        scan_node(node, diags, span, registry);
                    }
                }
            }
            IrNode::For { iter, body, .. } | IrNode::BridgeBatchFor { iter, body, .. } => {
                scan_expr(iter, diags, span, registry);
                for node in body {
                    scan_node(node, diags, span, registry);
                }
            }
            IrNode::While { cond, body } => {
                scan_expr(cond, diags, span, registry);
                for node in body {
                    scan_node(node, diags, span, registry);
                }
            }
            IrNode::Return(expr) => {
                if let Some(expr) = expr {
                    scan_expr(expr, diags, span, registry);
                }
            }
            IrNode::TryBlock {
                try_body,
                except_body,
                else_body,
                finally_body,
                ..
            } => {
                for node in try_body {
                    scan_node(node, diags, span, registry);
                }
                for node in except_body {
                    scan_node(node, diags, span, registry);
                }
                if let Some(nodes) = else_body {
                    for node in nodes {
                        scan_node(node, diags, span, registry);
                    }
                }
                if let Some(nodes) = finally_body {
                    for node in nodes {
                        scan_node(node, diags, span, registry);
                    }
                }
            }
            IrNode::Assert { test, msg } => {
                scan_expr(test, diags, span, registry);
                if let Some(expr) = msg {
                    scan_expr(expr, diags, span, registry);
                }
            }
            IrNode::Raise { message, cause, .. } => {
                scan_expr(message, diags, span, registry);
                if let Some(expr) = cause {
                    scan_expr(expr, diags, span, registry);
                }
            }
            IrNode::Expr(expr) => scan_expr(expr, diags, span, registry),
            IrNode::FuncDecl { body, .. } => {
                for node in body {
                    scan_node(node, diags, span, registry);
                }
            }
            IrNode::MethodDecl { name, body, .. } => {
                scan_magic_method(name, diags, span, registry);
                for node in body {
                    scan_node(node, diags, span, registry);
                }
            }
            IrNode::ImplBlock { methods, .. } => {
                for node in methods {
                    scan_node(node, diags, span, registry);
                }
            }
            IrNode::Sequence(nodes) => {
                for node in nodes {
                    scan_node(node, diags, span, registry);
                }
            }
            IrNode::Block { stmts } => {
                for node in stmts {
                    scan_node(node, diags, span, registry);
                }
            }
            IrNode::StructDef { .. }
            | IrNode::TypeAlias { .. }
            | IrNode::BridgeImport { .. }
            | IrNode::DynamicEnumDef { .. }
            | IrNode::Break
            | IrNode::Continue => {}
        }
    }

    for node in nodes {
        scan_node(node, &mut diags, &span, registry);
    }

    diags
}
