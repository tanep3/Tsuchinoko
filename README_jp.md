# Tsuchinoko 🐍➡️🦀

**Python to Rust トランスパイラ** - 型ヒント付きPythonコードをRustに変換

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[🇺🇸 English version](README.md)

## 概要

TsuchinokoはPythonの型ヒント付きコードをRustに変換するトランスパイラです。
Pythonの読みやすい構文でロジックを書き、Rustの安全性とパフォーマンスを得ることができます。

## 特徴

- ✅ **型ヒント活用**: `int`, `str`, `list[int]`, `tuple[int, str]` など
- ✅ **スライス型出力**: `&Vec<T>` ではなく `&[T]` を生成（Rustイディオム準拠）
- ✅ **所有権自動推論**: 参照渡し/所有渡しを自動判定
- ✅ **mut自動最小化**: 再代入がない変数は `mut` なしで宣言
- ✅ **snake_case変換**: `getOrder` → `get_order` 自動変換

## インストール

```bash
git clone https://github.com/TaneChannelTechnology/Tsuchinoko.git
cd Tsuchinoko
cargo build --release
cargo install --path .
```

## 使い方

```bash
# 基本的な変換
tnk your_file.py

# 出力先指定
tnk your_file.py -o output.rs

# Cargoプロジェクト生成
tnk your_file.py --project my_project

# チェックのみ（出力なし）
tnk your_file.py --check
```

### 入力例 (Python)

```python
def bubble_sort(lists: list[int]) -> tuple[list[int], int]:
    sorted_list: list[int] = list(lists)
    list_length: int = len(sorted_list)
    for i in range(list_length):
        for j in range(list_length - i - 1):
            if sorted_list[j] > sorted_list[j + 1]:
                temp: int = sorted_list[j]
                sorted_list[j] = sorted_list[j + 1]
                sorted_list[j + 1] = temp
    return sorted_list, list_length
```

### 出力例 (Rust)

```rust
fn bubble_sort(lists: &[i64]) -> (Vec<i64>, i64) {
    let mut sorted_list: Vec<i64> = lists.to_vec();
    let list_length: i64 = sorted_list.len() as i64;
    for i in 0..list_length {
        for j in 0..((list_length - i) - 1) {
            if sorted_list[j as usize] > sorted_list[(j + 1) as usize] {
                let temp: i64 = sorted_list[j as usize];
                sorted_list[j as usize] = sorted_list[(j + 1) as usize];
                sorted_list[(j + 1) as usize] = temp;
            }
        }
    }
    return (sorted_list, list_length);
}
```

## サポート機能

| Python構文 | Rust出力 | 状態 |
|-----------|---------|------|
| `x: int = 10` | `let x: i64 = 10;` | ✅ |
| `list[int]` | `Vec<i64>` / `&[i64]` | ✅ |
| `def func(x: int) -> int` | `fn func(x: i64) -> i64` | ✅ |
| `for i in range(n)` | `for i in 0..n` | ✅ |
| `if/elif/else` | `if/else if/else` | ✅ |
| `while` | `while` | ✅ |
| `list(x)` | `x.to_vec()` | ✅ |
| `len(x)` | `x.len()` | ✅ |
| `max(x)` | `x.iter().max().cloned().unwrap()` | ✅ |
| `x ** 2` | `x.pow(2)` | ✅ |
| `x.append(y)` | `x.push(y)` | ✅ |
| `x.extend(y)` | `x.extend(y)` | ✅ |

## ドキュメント

- [ユーザーマニュアル](docs/user-manual_jp.md)
- [デプロイガイド](docs/deploy-guide_jp.md)
- [要件定義書](docs/requirements.md)
- [システム設計書](docs/system-design.md)
- [API仕様書](docs/api-spec.md)

## テスト

```bash
cargo test
```

## プロジェクト構造

```
src/
├── lib.rs          # ライブラリエントリポイント
├── main.rs         # CLIエントリポイント
├── parser/         # Pythonパーサー (pest)
├── semantic/       # 意味解析 & 型推論
├── ir/             # 中間表現
├── emitter/        # Rustコード生成
└── error/          # エラー型
```

## ライセンス

MIT License

## 作者

**Tane Channel Technology**
