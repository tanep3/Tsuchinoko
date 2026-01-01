# Tsuchinoko 🐍➡️🦀

**Python to Rust トランスパイラ** - 型ヒント付きPythonコードをRustに変換

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-1.3.0-green.svg)](Cargo.toml)
[![Coverage](https://img.shields.io/badge/Python構文カバレッジ-62%25-blue.svg)](#機能ドキュメント)
[![Changelog](https://img.shields.io/badge/History-変更履歴-blue.svg)](CHANGELOG_jp.md)

[🇺🇸 English version](README.md)

## 概要

TsuchinokoはPythonの型ヒント付きコードをRustに変換するトランスパイラです。
Pythonの読みやすい構文でアルゴリズムロジックを書き、Rustの安全性とパフォーマンスを得ることができます。

> **カバレッジ**: Python構文機能の62%(60機能)をサポート。変数、演算子、制御フロー、関数、クラス、データ構造、エラー処理など、アルゴリズムプログラミングに必要な基本構造をカバーしています。

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
- **常駐プロセス方式** - `numpy` / `pandas` 等をIPC経由でサポート

## ベンチマーク 🚀

Python (3.x) と Tsuchinoko生成Rustコード (`rustc -O`でコンパイル) の速度比較です。
データ生成(LCG)とソート処理を含みます。

| アルゴリズム | データ数 (N) | Python | Tsuchinoko (Rust) | 高速化率 |
|-----------|---|--------|-------------------|---------|
| **Bubble Sort** | 10,000 | 5.394s | **0.037s** | **約146倍** 🚀 |
| **Radix Sort** | 10,000,000 | 8.908s | **0.278s** | **約32倍** 🚀 |

*`hyperfine` を使用してローカル環境 (Linux x86_64, V1.0.0) で計測。*

## インストール

```bash
git clone https://github.com/tanep3/Tsuchinoko.git
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

> [!NOTE]
> 外部ライブラリ (`import`) を使用する場合は、`--project` オプションを使用して依存関係を含む Cargo プロジェクトを作成してください。

> [!IMPORTANT]
> **venv 環境が必要です**: Resident Worker (NumPy/Pandas等) を使用するコードは、Python の仮想環境内で `tnk` を実行し、生成されたバイナリも同じ venv 環境内で実行してください。
> ```bash
> source venv/bin/activate
> tnk script.py --project my_app
> cd my_app && cargo run --release
> ```

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

- [サポート機能一覧](docs/supported_features_jp.md) | [Supported Features](docs/supported_features.md)
- [非サポート機能一覧](docs/unsupported_features_jp.md) | [Unsupported Features](docs/unsupported_features.md)

## 今後のロードマップ (Roadmap)

- [x] 名前付き引数サポート (`func(name="value")`)
- [x] デフォルト引数サポート (`def func(x=10)`)
- [x] 例外処理 (`try-except` → `catch_unwind`)
- [x] 可変長引数 (`*args` スプレッド演算子経由)
- [x] 常駐 Python ワーカーによる NumPy/Pandas サポート
- [x] ビット演算子 (`&`, `|`, `^`, `~`, `<<`, `>>`) (V1.3.0)
- [x] 組み込み関数 (`enumerate`, `zip`, `sorted`, `sum`, `all`, `any`, `map`, `filter`) (V1.3.0)
- [x] 辞書内包表記 (`{k: v for k, v in items}`) (V1.3.0)
- [x] `assert` 文 (V1.3.0)
- [ ] 完全な `**kwargs` サポート
- [ ] 完全なクラス継承サポート
- [ ] より多くの標準ライブラリマッピング

## ドキュメント

- [ユーザーマニュアル](docs/user-manual_jp.md) | [User Manual](docs/user-manual.md)
- [デプロイガイド](docs/deploy-guide_jp.md) | [Deploy Guide](docs/deploy-guide.md)

## ライセンス

MIT License

## 作者

**Tane Channel Technology**
