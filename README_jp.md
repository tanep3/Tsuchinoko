# Tsuchinoko 🐍➡️🦀

**Python to Rust トランスパイラ** - 型ヒント付きPythonコードをRustに変換

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-1.0.0-green.svg)](Cargo.toml)

[🇺🇸 English version](README.md)

## 概要

TsuchinokoはPythonの型ヒント付きコードをRustに変換するトランスパイラです。
Pythonの読みやすい構文でロジックを書き、Rustの安全性とパフォーマンスを得ることができます。

## 設計理念 (Design Philosophy)

Tsuchinokoは汎用的なPythonコンパイラではありません。以下の理念に基づいて設計されています：

- **人間が読めるロジックの維持**: 生成されたRustコードは可読性が高く、保守可能であることを目指します。
- **命令型Pythonから構造的Rustへの変換**: Pythonの制御フローをRustの等価な構造に直接マッピングします。
- **所有権よりも借用を優先**: 不要な割り当てを避けるため、可能な限り参照 (`&[T]`, `&str`) を自動的に使用します。

## 主要機能 ✨

- **基本構文** - 変数、型、演算子、制御フロー
- **高階関数** - 関数を引数として渡す、クロージャ
- **Lambda式** - `lambda x: x + 1` → `|x| x + 1`
- **基本クラス対応** - `__init__`とメソッドを持つ構造体的クラス
- **リスト内包表記** - `[x*2 for x in nums if x > 0]`
- **スマート型推論** - Auto-Ref, Auto-Deref, 最小`mut`

## ベンチマーク 🚀

Python (3.x) と Tsuchinoko生成Rustコード (`rustc -O`でコンパイル) の速度比較です。
データ生成(LCG)とソート処理を含みます。

| アルゴリズム | データ数 (N) | Python | Tsuchinoko (Rust) | 高速化率 |
|-----------|---|--------|-------------------|---------|
| **Bubble Sort** | 10,000 | 5.050s | **0.040s** | **約125倍** 🚀 |
| **Radix Sort** | 10,000,000 | 9.711s | **0.311s** | **約31倍** 🚀 |

*`hyperfine` を使用してローカル環境 (Linux x86_64) で計測。*

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

## 機能ドキュメント

詳細な機能一覧については以下を参照してください：

- [サポート機能一覧](docs/supported_features_ja.md) | [Supported Features](docs/supported_features.md)
- [非サポート機能一覧](docs/unsupported_features_ja.md) | [Unsupported Features](docs/unsupported_features.md)

## 今後のロードマップ (Roadmap)

- [ ] 完全なクラス継承サポート
- [ ] 例外処理 (`try-except` → `Result`)
- [ ] 名前付き引数サポート
- [ ] より多くの標準ライブラリマッピング

## ドキュメント

- [ユーザーマニュアル](docs/user-manual_jp.md) | [User Manual](docs/user-manual.md)
- [デプロイガイド](docs/deploy-guide_jp.md) | [Deploy Guide](docs/deploy-guide.md)
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
