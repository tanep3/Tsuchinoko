# Tsuchinoko API設計書

> **著者**: Tane Channel Technology  
> **バージョン**: 1.7.0  
> **最終更新**: 2026-01-15

---

## 1. CLI API

### 1.1 基本コマンド

```bash
tnk [OPTIONS] <INPUT>
```

### 1.2 引数

| 引数 | 必須 | 説明 |
|------|------|------|
| `<INPUT>` | Yes | 変換対象のPythonファイル |

### 1.3 オプション

| オプション | 短縮形 | デフォルト | 説明 |
|-----------|--------|-----------|------|
| `--output` | `-o` | `<INPUT>.rs` | 出力先Rustファイル |
| `--project` | `-p` | - | Rustプロジェクトフォルダを生成 |
| `--debug` | `-d` | false | デバッグ情報出力 |
| `--check` | `-c` | false | 変換可能性チェックのみ |
| `--diag-json` | - | false | JSON形式で診断情報を出力 (V1.7.0) |
| `--version` | `-V` | - | バージョン表示 |
| `--help` | `-h` | - | ヘルプ表示 |

### 1.4 使用例

```bash
# 基本変換
tnk input.py

# 出力先指定
tnk input.py -o output.rs

# デバッグモード
tnk input.py --debug

# 変換チェックのみ
tnk input.py --check

# プロジェクト生成
tnk input.py --project my_app
# 結果:
# my_app/
# ├── Cargo.toml
# ├── src/
# │   └── main.rs
# └── .gitignore
```

### 1.5 終了コード

| コード | 意味 |
|--------|------|
| 0 | 成功 |
| 1 | パースエラー |
| 2 | 型エラー |
| 3 | 未対応構文エラー |
| 4 | IOエラー |

---

## 2. ライブラリ API

### 2.1 公開モジュール

```rust
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod ir;
pub mod emitter;
pub mod error;
pub mod bridge;  // V1.2.0: 外部ライブラリ連携
```

### 2.2 主要関数

#### 2.2.1 トップレベル変換

```rust
/// Pythonソースコードを Rustソースコードに変換
pub fn transpile(source: &str) -> Result<String, TsuchinokoError>;

/// ファイルを変換
pub fn transpile_file(input: &Path, output: &Path) -> Result<(), TsuchinokoError>;
```

#### 2.2.2 パーサー

```rust
/// Pythonソースをパースし ASTを返す
pub fn parse(source: &str) -> Result<Ast, ParseError>;
```

#### 2.2.3 意味解析

```rust
/// ASTを解析し 型付きASTを返す
pub fn analyze(ast: &Ast) -> Result<TypedAst, SemanticError>;
```

#### 2.2.4 エミッター

```rust
/// 型付きASTからRustコードを生成
pub fn emit(typed_ast: &TypedAst) -> String;
```

---

## 3. エラー API

### 3.1 トランスパイラエラー型

```rust
#[derive(Debug, thiserror::Error)]
pub enum TsuchinokoError {
    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
    
    #[error("Type error at line {line}: {message}")]
    TypeError { line: usize, message: String },
    
    #[error("Undefined variable '{name}' at line {line}")]
    UndefinedVariable { name: String, line: usize },
    
    #[error("Unsupported syntax at line {line}: {syntax}")]
    UnsupportedSyntax { syntax: String, line: usize },
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### 3.2 ランタイムエラー型 (V1.5.2 生成コード側)

```rust
/// 生成された Rust コード内で使用されるエラー型
pub struct TsuchinokoError {
    kind: String,           // 例外型名 ("ValueError", "RuntimeError" など)
    message: String,        // エラーメッセージ
    line: usize,            // Python ソース行番号
    cause: Option<Box<TsuchinokoError>>,  // 例外チェーン (raise from)
}

impl TsuchinokoError {
    /// 行番号と cause 付きで生成
    pub fn with_line(kind: &str, message: &str, line: usize, cause: Option<TsuchinokoError>) -> Self;
}

impl std::fmt::Display for TsuchinokoError {
    // 出力例: "[line 10] ValueError: invalid value"
    //         "Caused by: [line 5] RuntimeError: original error"
}
```

### 3.3 エラー出力形式

```
error[E0001]: Parse error at line 5
  --> input.py:5:10
   |
 5 |     x = @@invalid
   |         ^^ unexpected token
```

---

## 4. 拡張 API

### 4.1 カスタム変換ルール

```rust
/// カスタム型マッピング
pub trait TypeMapper {
    fn map_type(&self, py_type: &str) -> Option<String>;
}

/// カスタムエミッター
pub trait CustomEmitter {
    fn emit_expr(&self, expr: &IrExpr) -> Option<String>;
    fn emit_stmt(&self, stmt: &IrNode) -> Option<String>;
}
```

### 4.2 フック

```rust
/// パース後フック
pub fn set_post_parse_hook(hook: fn(&Ast) -> Ast);

/// 型解析後フック
pub fn set_post_analyze_hook(hook: fn(&TypedAst) -> TypedAst);
```

---

## 5. デバッグ API

### 5.1 デバッグ出力

```rust
/// AST をデバッグ形式で出力
pub fn debug_ast(ast: &Ast) -> String;

/// 型情報をデバッグ形式で出力
pub fn debug_types(typed_ast: &TypedAst) -> String;

/// IR をデバッグ形式で出力
pub fn debug_ir(ir: &[IrNode]) -> String;
```

### 5.2 デバッグオプション

```bash
tnk input.py --debug

# 出力例:
# [DEBUG] Parsing input.py...
# [DEBUG] AST:
#   FuncDef { name: "main", ... }
# [DEBUG] Type inference...
# [DEBUG] Types:
#   x: i64
#   y: Vec<i64>
# [DEBUG] Emitting Rust code...
```
---

## 6. Python Worker RPC Protocol (V1.7.0)

> [!IMPORTANT]
> V1.7.0で導入された Remote Object Handle パターンの通信プロトコル仕様。

### 6.1 プロトコル基礎

- **フォーマット**: NDJSON (Newline Delimited JSON) over Stdin/Stdout
- **エンコーディング**: UTF-8
- **セッション**: 全てのコマンドは `session_id` を保持
- **リクエストID**: 任意の `req_id` を付与可能（レスポンスに同じIDが返される）

### 6.2 データ型 (Tagged Union)

全ての値は `TnkValue` として表現される：

```typescript
type TnkValue =
  | { kind: "value", value: number | string | boolean | null }
  | { kind: "handle", id: string, type: string, repr: string, session_id: string }
  | { kind: "list", items: TnkValue[] }
  | { kind: "tuple", items: TnkValue[] }
  | { kind: "dict", items: {key: TnkValue, value: TnkValue}[] };
```

### 6.3 コマンドセット (Maximum A)

#### `call_method`
オブジェクトのメソッドを呼び出す。

```json
{
  "cmd": "call_method",
  "session_id": "uuid-...",
  "target": "handle_id",
  "method": "method_name",
  "args": [ { ...TnkValue... }, ... ]
}
```

#### `get_attribute`
プロパティや属性を取得。**制限**: `_`で始まる属性は禁止。

```json
{
  "cmd": "get_attribute",
  "session_id": "uuid-...",
  "target": "handle_id",
  "name": "attribute_name"
}
```

#### `get_item`
インデックスまたはキーで要素を取得 (`obj[key]`)。

```json
{
  "cmd": "get_item",
  "session_id": "uuid-...",
  "target": "handle_id",
  "key": { ...TnkValue... }
}
```

#### `slice`
スライスを取得 (`obj[start:stop:step]`)。

```json
{
  "cmd": "slice",
  "session_id": "uuid-...",
  "target": "handle_id",
  "start": { ...TnkValue... },
  "stop": { ...TnkValue... },
  "step": { ...TnkValue... }
}
```

#### `iter` / `iter_next_batch`
イテレータ操作。`iter_next_batch`は`batch_size`単位で要素を取得（IPC削減）。

```json
{
  "cmd": "iter_next_batch",
  "session_id": "uuid-...",
  "target": "iterator_handle_id",
  "batch_size": 1000
}
```

#### `delete`
オブジェクトをストアから解放。**Idempotent（冪等）**。

### 6.4 レスポンス形式

#### 成功
```json
{
  "kind": "ok",
  "req_id": "optional-id-1",
  "value": { ...TnkValue... },
  "meta": { "done": boolean }  // オプション
}
```

#### エラー
```json
{
  "kind": "error",
  "req_id": "optional-id-1",
  "error": {
    "code": "ErrorCode",
    "py_type": "ValueError",
    "message": "エラー詳細...",
    "traceback": "Traceback文字列...",
    "op": { ...元のコマンド... }
  }
}
```

### 6.5 エラーコード

| コード | 説明 |
|---|---|
| `ProtocolError` | JSON形式不正、必須フィールド欠落、未知のコマンド |
| `StaleHandle` | Handle IDがセッションストアに見つからない |
| `WorkerCrash` | Workerプロセスが予期せず終了 |
| `ValueTooLarge` | `.to_value()`の結果がサイズ制限超過 |
| `SecurityViolation` | `_`属性へのアクセス、`eval`、`exec`など禁止操作 |
| `PythonException` | 標準的なPython例外 (IndexError, KeyError, etc.) |
| `TypeMismatch` | サポートされていない操作 |

詳細は `docs/old_versions/v1.7.0_api_spec.md` を参照。
