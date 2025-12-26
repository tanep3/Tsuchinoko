# Tsuchinoko ユーザーマニュアル

## 目次

1. [インストール](#インストール)
2. [基本的な使い方](#基本的な使い方)
3. [コマンドラインオプション](#コマンドラインオプション)
4. [対応するPythonコードの書き方](#対応するpythonコードの書き方)
5. [型ヒントリファレンス](#型ヒントリファレンス)
6. [制限事項](#制限事項)

---

## インストール

### 前提条件

- Rust 1.70 以降
- Cargo（Rustに同梱）

### ソースからインストール

```bash
git clone https://github.com/TaneChannelTechnology/Tsuchinoko.git
cd Tsuchinoko
cargo build --release
cargo install --path .
```

インストール後、`tnk` コマンドがグローバルで使用可能になります。

---

## 基本的な使い方

### 単一ファイルの変換

```bash
tnk your_file.py
```

出力先: カレントディレクトリの `your_file.rs`

### 出力先を指定

```bash
tnk your_file.py -o custom_output.rs
```

### Cargoプロジェクトの生成

```bash
tnk your_file.py --project my_project
```

以下の構造のCargoプロジェクトが生成されます：
```
my_project/
├── Cargo.toml
├── .gitignore
└── src/
    └── main.rs
```

### チェックモード

```bash
tnk your_file.py --check
```

出力を生成せずにPythonコードを検証します。

---

## コマンドラインオプション

| オプション | 短縮形 | 説明 |
|-----------|--------|------|
| `--output` | `-o` | 出力ファイルパスを指定 |
| `--project` | `-p` | Cargoプロジェクトを生成 |
| `--check` | `-c` | チェックのみ（出力なし） |
| `--debug` | `-d` | デバッグ情報を表示 |
| `--help` | `-h` | ヘルプを表示 |
| `--version` | `-V` | バージョンを表示 |

---

## 対応するPythonコードの書き方

### 必須: 型ヒント

すべての変数と関数シグネチャに型ヒントが**必須**です：

```python
# ✅ OK
x: int = 10
def add(a: int, b: int) -> int:
    return a + b

# ❌ NG（型ヒントなし）
x = 10
def add(a, b):
    return a + b
```

### エントリポイント

標準的なPythonエントリポイントパターンを使用してください：

```python
def main() -> None:
    # your code here
    pass

if __name__ == "__main__":
    main()
```

これにより適切なRustの `main()` 関数が生成されます。

---

## 型ヒントリファレンス

| Python型 | Rust型 |
|----------|--------|
| `int` | `i64` |
| `float` | `f64` |
| `str` | `String` |
| `bool` | `bool` |
| `list[T]` | `Vec<T>` |
| `tuple[T, U]` | `(T, U)` |
| `None` | `()` |

### 関数パラメータ

リストパラメータは自動的に参照渡しになります：

```python
def process(data: list[int]) -> int:  # dataは &[i64] になる
    return len(data)
```

---

## 制限事項

### 未対応

- `Optional` 型
- スライシング (`arr[1:3]`, `arr[-1]`)
- `break` / `continue`
- 例外処理 (`try`/`except`)
- クラスとOOP
- ジェネレータと `yield`
- `async`/`await`

### エッジケース

- 空リストでの `max()` はpanicする（Pythonと同様）
- 負の数を含むradixソートは未対応

---

## サンプル

`examples/` ディレクトリに動作するサンプルがあります：

- `bubbleSort.py` - バブルソート実装
- `recursiveRadixSort.py` - 基数ソート実装
