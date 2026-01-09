# サポート機能一覧

Tsuchinokoトランスパイラが現在サポートしているPython機能の一覧です。

## 基本構文

- **変数宣言** 型ヒント付き (`x: int = 10`)
- **基本型**: `int`, `float`, `str`, `bool`, `None`
- **コレクション型**: `list[T]`, `dict[K, V]`, `tuple[...]`
- **Optional型**: `Optional[T]`, `T | None`
- **Optionalパターン**: `x or default` → `unwrap_or`, 三項演算子+Noneチェック (V1.5.0)
- **算術演算子**: `+`, `-`, `*`, `/`, `//`, `%`, `**`, `@` (V1.3.0)
- **比較演算子**: `==`, `!=`, `<`, `>`, `<=`, `>=`
- **連鎖比較** (`0 < x < 10` → `0 < x && x < 10`) (V1.6.0)
- **論理演算子**: `and`, `or`, `not`
- **包含演算子**: `in`, `not in` (V1.3.0)
- **同一性演算子**: `is`, `is not` (`None`との比較)
- **ビット演算子**: `&`, `|`, `^`, `~`, `<<`, `>>` (V1.3.0)
- **累算代入演算子**: `+=`, `-=`, `*=`, `/=`, `//=`, `%=`, `**=`, `&=`, `|=`, `^=`, `<<=`, `>>=` (V1.3.0)
- **Docstring**: 三重cote文字列をRustコメントに変換

## 制御フロー

- **If/elif/else** 文
- **Forループ** (`range()`、コレクション反復)
- **Whileループ**
- **Break/Continue** 文
- **条件式** (`x if cond else y`)
- **アーリーリターン** (V1.2.0 改善)

## 関数

- **関数定義** 型ヒント付き
- **Return文** オプショナル値付き
- **再帰** サポート
- **ネスト関数** (Rustクロージャに変換)
- **Lambda式** (`lambda x: x + 1`)
- **高階関数** (関数を引数として渡す)
- **名前付き引数** (`func(name="value")`)
- **デフォルト引数** (`def func(x=10)`) (V1.2.0)
- **\*\*kwargs** (`def func(**kwargs)` → `HashMap<String, Value>`) (V1.6.0)

## データ構造

- **リストリテラル** と操作
- **リスト内包表記** (基本と条件付き)
- **辞書内包表記** (`{k: v for k, v in items}`) (V1.3.0)
- **辞書リテラル** と操作
- **タプルリテラル** とアンパック
- **Setリテラル** (`{1, 2, 3}` → `HashSet`) (V1.5.0)
- **Set内包表記** (`{x*2 for x in nums}` → `HashSet`) (V1.6.0)
- **構造体定義** (クラス構文経由)
- **負のインデックス** (`nums[-1]`)
- **スライス記法** (`[:3]`, `[-3:]`, `[1:n-1]`)
- **ステップスライス** (`[::2]`, `[::-1]`) (V1.5.0)
- **インデックススワップ** (`a[i], a[j] = a[j], a[i]` → `a.swap()`)
- **リストのコピー** (`l.copy()` → `l.to_vec()`) (V1.2.0)
- **多重代入** (`a, b, c = 1, 2, 3`) (V1.3.0)
- **Listメソッド**: `pop`, `insert`, `remove`, `extend`, `clear` (V1.5.0)
- **Dictメソッド**: `keys`, `values`, `get`, `pop`, `update` (V1.5.0)
- **Setメソッド**: `add`, `remove`, `discard`, `union`, `intersection` (V1.5.0)

## クラス & オブジェクト

- **基本クラス定義** `__init__`付き
- **インスタンス属性** (`self.attr`)
- **メソッド定義**
- **静的メソッド** (`@staticmethod`)
- **Dataclass** (`@dataclass`) (V1.2.0 部分対応)
- **単一継承** (`class Child(Parent)`) → コンポジション (V1.6.0)
- **super()呼び出し** (`super().method()` → `self.base.method()`) (V1.6.0)
- **@propertyデコレータ** → getter/setterメソッド (V1.6.0)

## リソース管理 (V1.6.0)

- **with文** → RAIIスコープ (`with open(...) as f:` → `{ let f = ...; }`)
- RustのDroptraitによる自動リソース解放

## 常駐プロセス方式 (Resident Python Worker) (V1.2.0) 🆕

Tsuchinoko V1.2.0 では、Rust への直接変換が困難なライブラリをサポートするために常駐 Python ワーカーを導入しました。

- **NumPy** (`import numpy as np`)
- **Pandas** (`import pandas as pd`)
- **OpenCV** (`import cv2`) (V1.4.0)
- **SciPy**
- その他、Python 環境で利用可能なライブラリ（IPC経由）

### `from` インポート構文 (V1.4.0) 🆕

- **関数の直接インポート**: `from numpy import mean, std`
- `py_bridge.call_json("numpy.mean", ...)` 呼び出しに自動変換

### 永続オブジェクトハンドル 🆕

Python オブジェクトをブリッジ呼び出しをまたいで保持できるようになりました：
- **複雑なオブジェクト状態**: DataFrame や NumPy 配列、カスタムクラスインスタンスをメモリに保持
- **メソッドチェーン**: 同じオブジェクトハンドルに対して複数のメソッドを呼び出し可能
- **インデックスアクセス**: ハンドル経由で Python オブジェクト要素に直接アクセス (`df["column"]`)
- **ハンドル連携**: ハンドルを別の Python ライブラリ関数にシームレスに渡すことが可能

## 組み込み関数

- `len()` - 長さ取得
- `range()` - 数値範囲反復
- `print()` - コンソール出力 (f-string debug `"{x=}"` / `"{:?}"` 対応)
- `list()` - リスト変換
- `min()`, `max()` - 最小/最大値
- `abs()` - 絶対値
- `int()`, `float()`, `str()`, `bool()` - 型変換
- `enumerate()` - インデックス付きイテレーション (V1.3.0)
- `zip()` - 並列イテレーション (V1.3.0)
- `sorted()` - ソート済みリスト生成 (V1.3.0)
- `reversed()` - 逆順イテレーション (V1.3.0)
- `sum()` - 合計計算 (V1.3.0)
- `all()`, `any()` - 全要素/任意要素の真偽判定 (V1.3.0)
- `map()`, `filter()` - 関数型イテレータ変換 (V1.3.0)
- `assert` - アサーション文 (V1.3.0)
- `input()` - ユーザー入力（プロンプト付き） (V1.5.0)
- `round()` - 四捨五入（精度指定可） (V1.5.0)
- `chr()`, `ord()` - 文字・コードポイント変換 (V1.5.0)
- `bin()`, `hex()`, `oct()` - 数値フォーマット変換 (V1.5.0)
- `isinstance()` - 型チェック → `DynamicValue` enum + `match` (V1.6.0)

## Mathモジュール (V1.3.0 / V1.4.0)

- **関数**: `math.sqrt`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `exp`, `log`, `log10`, `log2`, `abs`, `floor`, `ceil`, `round`
- **定数 (V1.4.0)**: `math.pi`, `math.e`, `math.tau`, `math.inf`, `math.nan` → Rust ネイティブ定数に変換

## 文字列機能

- **文字列リテラル** (シングル/ダブルクォート)
- **F文字列** (`f"Hello {name}"`)
  - デバッグフォーマット `"{x=}"` / `"{:?}"` 対応 (V1.2.0)
- **文字列メソッド**: `.upper()`, `.lower()`, `.strip()`, `.split()`, `.join()` など
- **文字列メソッド (V1.5.0)**: `.replace()`, `.startswith()`, `.endswith()`, `.find()`, `.rfind()`, `.index()`, `.count()`
- **文字列判定 (V1.5.0)**: `.isdigit()`, `.isalpha()`, `.isalnum()`

## エラー処理

- **try/except** ブロック (`catch_unwind`に変換)
- **複数例外型** (`except (ValueError, TypeError):`) (V1.5.0)
- **例外変数** (`except ValueError as e:`) (V1.5.0)
- **try/except/finally** ブロック (V1.5.0)
- **try/except/else** ブロック (例外なし時のみ `else` 実行) (V1.5.2)
- **raise** 文 (`Err(TsuchinokoError)` または `panic!` に変換)
- **raise from** (`raise A from B`) - 例外チェーン (`cause` 保持) (V1.5.2)
- **Result型統一** - 例外発生関数は `Result<T, TsuchinokoError>` を返す (V1.5.2)
- **エラー行番号** - Python ソース行番号をエラーに含む (V1.5.2)
- **ValueError**, **TypeError** (`TsuchinokoError`に変換)

## 型システム

- **型エイリアス** (`MyType = list[int]`)
- **Callable型** (`Callable[[T], U]`)
- **関数型推論**
- **自動型強制** (Auto-Ref, Auto-Deref, Auto-Clone)
- **Type Narrowing** (`if x is None` / `if x is not None`)

## 実験的機能: PyO3 直接連携

> [!NOTE]
> PyO3 の直接呼び出しも引き続きサポートされますが、互換性のために **Resident Worker** の使用を推奨します。
