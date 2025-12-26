# Tsuchinoko システム設計書

> **著者**: Tane Channel Technology  
> **バージョン**: 0.5.0  
> **最終更新**: 2025-12-26

---

## 1. システム概要

Tsuchinokoは、型ヒント付きPythonコードをRustコードへ変換するトランスパイラである。
Rust + pestで実装し、単一バイナリとして配布可能。

### 1.1 設計思想

> **たねちゃんの哲学**
> - Python = 処理を記述する言語
> - Rust = 構造を記述する言語

この思想に基づき、Pythonの処理記述を解析し、Rustの構造的なコードへ変換する。

---

## 2. アーキテクチャ

### 2.1 全体構成

```mermaid
flowchart TB
    subgraph Input
        PY[Python Source<br/>.py]
    end
    
    subgraph Tsuchinoko
        LEX[Lexer<br/>字句解析]
        PARSE[Parser<br/>pest構文解析]
        AST[AST<br/>抽象構文木]
        SEM[Semantic Analyzer<br/>意味解析・型推論]
        IR[IR<br/>中間表現]
        EMIT[Emitter<br/>Rustコード生成]
    end
    
    subgraph Output
        RS[Rust Source<br/>.rs]
    end
    
    PY --> LEX --> PARSE --> AST --> SEM --> IR --> EMIT --> RS
```

### 2.2 処理パイプライン

| Phase | モジュール | 入力 | 出力 | 説明 |
|-------|-----------|------|------|------|
| 1 | Lexer | ソースコード | トークン列 | 字句解析 |
| 2 | Parser | トークン列 | Parse Tree | pest文法でパース |
| 3 | AST Builder | Parse Tree | AST | 抽象構文木構築 |
| 4 | Semantic | AST | Typed AST | 型推論・スコープ解決 |
| 5 | IR Generator | Typed AST | IR | 中間表現生成 |
| 6 | Emitter | IR | Rust Code | コード出力 |

---

## 3. モジュール構成

### 3.1 ディレクトリ構造

```
tsuchinoko/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLIエントリポイント
│   ├── lib.rs               # ライブラリルート
│   ├── lexer/
│   │   ├── mod.rs
│   │   └── token.rs         # トークン定義
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── python.pest      # Python文法定義
│   │   └── ast.rs           # AST構造体
│   ├── semantic/
│   │   ├── mod.rs
│   │   ├── scope.rs         # スコープ管理
│   │   ├── types.rs         # 型システム
│   │   └── inference.rs     # 型推論
│   ├── ir/
│   │   ├── mod.rs
│   │   └── nodes.rs         # IR構造体
│   ├── emitter/
│   │   ├── mod.rs
│   │   └── rust.rs          # Rust出力
│   └── error.rs             # エラー定義
├── tests/
│   ├── lexer_tests.rs
│   ├── parser_tests.rs
│   ├── semantic_tests.rs
│   └── integration_tests.rs
└── examples/
    └── sample.py            # サンプル入力
```

### 3.2 コンポーネント図

```mermaid
graph TB
    subgraph CLI["CLI Layer"]
        MAIN[main.rs<br/>clap]
    end
    
    subgraph Core["Core Library"]
        LIB[lib.rs]
        LEX[lexer/]
        PARSE[parser/]
        SEM[semantic/]
        IR[ir/]
        EMIT[emitter/]
        ERR[error.rs]
    end
    
    subgraph External["External Crates"]
        PEST[pest]
        CLAP[clap]
        THISERR[thiserror]
    end
    
    MAIN --> LIB
    LIB --> LEX
    LIB --> PARSE
    LIB --> SEM
    LIB --> IR
    LIB --> EMIT
    PARSE --> PEST
    MAIN --> CLAP
    ERR --> THISERR
```

---

## 4. データ構造

### 4.1 トークン

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // リテラル
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    
    // 識別子・キーワード
    Ident(String),
    Keyword(Keyword),
    
    // 演算子
    Operator(Operator),
    
    // 区切り
    Delimiter(Delimiter),
    
    // インデント
    Indent,
    Dedent,
    Newline,
}
```

### 4.2 AST

```rust
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    BinOp { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    UnaryOp { op: UnaryOp, operand: Box<Expr> },
    Call { func: Box<Expr>, args: Vec<Expr> },
    Index { target: Box<Expr>, index: Box<Expr> },
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign { targets: Vec<String>, value: Expr, type_hint: Option<TypeHint> },
    AugAssign { target: String, op: BinOp, value: Expr },
    FuncDef { name: String, params: Vec<Param>, return_type: Option<TypeHint>, body: Vec<Stmt> },
    If { condition: Expr, then_body: Vec<Stmt>, else_body: Option<Vec<Stmt>> },
    For { target: String, iter: Expr, body: Vec<Stmt> },
    While { condition: Expr, body: Vec<Stmt> },
    Return(Option<Expr>),
    Expr(Expr),
}
```

### 4.3 型システム

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    List(Box<Type>),
    Tuple(Vec<Type>),
    Dict(Box<Type>, Box<Type>),
    Optional(Box<Type>),
    Func { params: Vec<Type>, ret: Box<Type> },
    Unknown,
}
```

### 4.4 IR

```rust
#[derive(Debug, Clone)]
pub enum IrNode {
    VarDecl { name: String, ty: Type, mutable: bool, init: Option<Box<IrNode>> },
    Assign { target: String, value: Box<IrNode> },
    FuncDecl { name: String, params: Vec<(String, Type)>, ret: Type, body: Vec<IrNode> },
    If { cond: Box<IrNode>, then_block: Vec<IrNode>, else_block: Option<Vec<IrNode>> },
    For { var: String, iter: Box<IrNode>, body: Vec<IrNode> },
    While { cond: Box<IrNode>, body: Vec<IrNode> },
    Return(Option<Box<IrNode>>),
    Expr(IrExpr),
}
```

---

## 5. pest文法設計

### 5.1 基本構造

```pest
// python.pest - Phase 1 文法

program = { SOI ~ statement* ~ EOI }

statement = {
    func_def |
    if_stmt |
    for_stmt |
    while_stmt |
    assign_stmt |
    expr_stmt |
    return_stmt
}

func_def = {
    "def" ~ ident ~ "(" ~ param_list? ~ ")" ~ type_annotation? ~ ":" ~ block
}

if_stmt = {
    "if" ~ expr ~ ":" ~ block ~ elif_clause* ~ else_clause?
}

for_stmt = {
    "for" ~ ident ~ "in" ~ expr ~ ":" ~ block
}

assign_stmt = {
    target ~ type_annotation? ~ "=" ~ expr
}

expr = { comparison }
comparison = { arith ~ (comp_op ~ arith)* }
arith = { term ~ (add_op ~ term)* }
term = { factor ~ (mul_op ~ factor)* }
factor = { unary_op? ~ primary }
primary = { literal | ident | "(" ~ expr ~ ")" | list | call }

// 型ヒント
type_annotation = { ":" ~ type_expr }
type_expr = { ident ~ ("[" ~ type_expr ~ "]")? }

// トークン
ident = @{ ASCII_ALPHA ~ (ASCII_ALPHANUMERIC | "_")* }
int_literal = @{ ASCII_DIGIT+ }
float_literal = @{ ASCII_DIGIT+ ~ "." ~ ASCII_DIGIT+ }
string_literal = @{ "\"" ~ (!"\"" ~ ANY)* ~ "\"" }

WHITESPACE = _{ " " | "\t" }
COMMENT = _{ "#" ~ (!NEWLINE ~ ANY)* }
```

---

## 6. 変換ルール

### 6.1 型マッピング

| Python型ヒント | Rust型 |
|---------------|--------|
| `int` | `i64` |
| `float` | `f64` |
| `str` | `String` |
| `bool` | `bool` |
| `list[T]` | `Vec<T>` |
| `tuple[T, U]` | `(T, U)` | 型ヒント必須 |
| `dict[K, V]` | `HashMap<K, V>` | |
| `Optional[T]` | `Option<T>` | |
| `None` | `()` | |

### 6.2 構文マッピング

| Python | Rust |
|--------|------|
| `def func(x: int) -> int:` | `fn func(x: i64) -> i64 {` |
| `x: int = 10` | `let x: i64 = 10;` |
| `x = 10` | `x = 10;` |
| `if cond:` | `if cond {` |
| `for i in range(n):` | `for i in 0..n {` |
| `for item in items:` | `for item in items.iter() {` |
| `while cond:` | `while cond {` |
| `return x` | `return x;` |

### 6.3 シーケンス図

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Parser
    participant Semantic
    participant Emitter
    
    User->>CLI: tnk input.py
    CLI->>Parser: parse(source)
    Parser->>Parser: pest parse
    Parser-->>CLI: AST
    CLI->>Semantic: analyze(ast)
    Semantic->>Semantic: type inference
    Semantic->>Semantic: scope resolution
    Semantic-->>CLI: Typed AST
    CLI->>Emitter: emit(typed_ast)
    Emitter-->>CLI: Rust code
    CLI-->>User: output.rs
```

---

## 7. エラーハンドリング

### 7.1 エラー種別

```rust
#[derive(Debug, thiserror::Error)]
pub enum TsuchinokoError {
    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
    
    #[error("Type error: {message}")]
    TypeError { message: String },
    
    #[error("Undefined variable: {name}")]
    UndefinedVariable { name: String },
    
    #[error("Unsupported syntax: {syntax}")]
    UnsupportedSyntax { syntax: String },
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

---

## 8. 依存クレート

| クレート | バージョン | 用途 |
|---------|-----------|------|
| pest | 2.x | PEGパーサー |
| pest_derive | 2.x | 文法マクロ |
| clap | 4.x | CLI引数解析 |
| thiserror | 1.x | エラー定義 |
| anyhow | 1.x | エラー伝播 |

---

## 9. 非機能要件対応

| NFR-ID | 対応方法 |
|--------|----------|
| PERF-001 | Rustネイティブ速度、ゼロコピーパース |
| REL-001 | thiserrorで構造化エラー、行番号付き |
| MAIN-001 | モジュール分離、pest文法分離 |
| USA-001 | clap使用、--help自動生成 |
| TEST-001 | cargo test + tarpaulin |

---

## 10. 参考資料

- 旧Python実装: `src_v0.5/`
- 旧ドキュメント: `docs_old/`
- pest公式: https://pest.rs/
