# Tsuchinoko ユーザーマニュアル

## 目次

1. [クイックスタート](#クイックスタート)
2. [インストール](#インストール)
3. [基本的な使い方](#基本的な使い方)
4. [コマンドラインオプション](#コマンドラインオプション)
5. [対応するPythonコードの書き方](#対応するpythonコードの書き方)
6. [よく使うパターン](#よく使うパターン)
7. [型ヒントリファレンス](#型ヒントリファレンス)
8. [制限事項](#制限事項)

---

## クイックスタート

> **所要時間**: 5分

`hello.py` を作成:

```python
def greet(name: str) -> str:
    return f"Hello, {name}!"

def main():
    message: str = greet("Tsuchinoko")
    print(message)

main()
```

トランスパイルして実行:

```bash
tnk hello.py -o hello.rs
rustc hello.rs -o hello
./hello
```

出力:
```
Hello, Tsuchinoko!
```

---

## インストール

### 前提条件

- Rust 1.70 以降
- Cargo（Rustに同梱）

### ソースからインストール

```bash
git clone https://github.com/tanep3/Tsuchinoko.git
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

外部ライブラリ (NumPy, Pandas など) を使う場合:

```bash
# まず venv を有効化
source venv/bin/activate

tnk your_file.py --project my_project
cd my_project
cargo run --release
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

## よく使うパターン

### リスト

```python
nums: list[int] = [1, 2, 3, 4, 5]
doubled: list[int] = [x * 2 for x in nums]
nums.append(6)
first: int = nums.pop(0)
```

### 辞書

```python
scores: dict[str, int] = {"Alice": 90, "Bob": 85}
alice_score: int = scores["Alice"]
scores["Charlie"] = 88
for key in scores.keys():
    print(key)
```

### セット (v1.5.0)

```python
s: set[int] = {1, 2, 3}
s.add(4)
s.remove(1)
union: set[int] = s | {5, 6}
```

### Optional 値

```python
from typing import Optional

def find(items: list[int], target: int) -> Optional[int]:
    for i, item in enumerate(items):
        if item == target:
            return i
    return None

result: Optional[int] = find([1, 2, 3], 2)
value: int = result or -1  # x or default パターン
```

### スライス (v1.5.0)

```python
nums: list[int] = [0, 1, 2, 3, 4, 5]
first_three: list[int] = nums[:3]
reversed_nums: list[int] = nums[::-1]
every_other: list[int] = nums[::2]
```

### 例外処理 (v1.5.0)

```python
try:
    result: int = int("abc")
except ValueError as e:
    print("Invalid input")
finally:
    print("Cleanup")
```

---

## 型ヒントリファレンス

| Python型 | Rust型 |
|----------|--------|
| `int` | `i64` |
| `float` | `f64` |
| `str` | `String` |
| `bool` | `bool` |
| `list[T]` | `Vec<T>` |
| `dict[K, V]` | `HashMap<K, V>` |
| `set[T]` | `HashSet<T>` |
| `tuple[T, U]` | `(T, U)` |
| `Optional[T]` | `Option<T>` |
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

- `**kwargs` (キーワード可変長引数)
- 複雑なクラス継承
- ジェネレータと `yield`
- `async`/`await`
- カスタム例外クラス
- `raise ... from ...` (v1.5.1 で対応予定)
- `try/except/else` (v1.5.1 で対応予定)

### エッジケース

- 空リストでの `max()` はpanicする（Pythonと同様）
- 負の数を含むradixソートは未対応

---

## サンプル

`examples/` ディレクトリに動作するサンプルがあります：

- `examples/simple/` - 基本的な変換サンプル (54ファイル)
- `examples/import/` - 外部ライブラリサンプル (8ファイル)
- `examples/benchmarks/` - パフォーマンスベンチマーク

---

## 関連ドキュメント

- [サポート機能一覧](supported_features_jp.md)
- [非サポート機能一覧](unsupported_features_jp.md)
