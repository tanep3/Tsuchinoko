# Tsuchinoko API設計書

> **著者**: Tane Channel Technology  
> **バージョン**: 0.5.0  
> **最終更新**: 2025-12-26

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

### 3.1 エラー型

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

### 3.2 エラー出力形式

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
