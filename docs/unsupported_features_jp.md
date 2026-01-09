# 非サポート機能一覧

Tsuchinokoトランスパイラが現在サポートしていないPython機能の一覧です。

## 言語構造

### 文

- **`del` 文** (変数・要素の削除)
- **`match` 文** (Python 3.10+ パターンマッチング)
- **`type` 文** (Python 3.12+ 型エイリアス構文)
- **`global` 文** (グローバル変数宣言)
- **`nonlocal` 文** (ネスト関数の変数バインディング)
- **Walrus 演算子** (`:=` 代入式)

### Async/Await

- **`async def`** (コルーチン定義)
- **`await`** 式
- **`async for`** (非同期イテレーション)
- **`async with`** (非同期コンテキストマネージャ)

### ジェネレータ

- **`yield` 文** (ジェネレータ関数)
- **`yield from`** (ジェネレータ委譲)
- **ジェネレータ式** (`(x for x in items)`)

### 内包表記

> [!NOTE]
> リスト内包表記、辞書内包表記、セット内包表記 (V1.6.0) はサポートされています。

### コンテキストマネージャ

- **カスタムコンテキストマネージャ** (`__enter__` / `__exit__` プロトコル)

> [!NOTE]
> `with open(...) as f:` (ファイル操作) はサポートされています (V1.6.0)。

### 引数

> [!NOTE]
> `*args` (位置可変長引数) および `**kwargs` (キーワード可変長引数) はサポートされています (V1.6.0)。

## 例外処理

- **カスタム例外クラス** (独自の例外型定義)

## クラス機能

### 継承とOOP

- **クラス継承** (多重継承を除く)
- **多重継承**
- **抽象基底クラス** (`abc` モジュール)
- **メタクラス**

### デコレータ

- **デコレータ** (`@staticmethod`, `@dataclass`, `@property` 以外)
- **クラスメソッド** (`@classmethod`)

> [!NOTE]
> 単一継承、`super()`、`@property` はサポートされています (V1.6.0)。

### マジックメソッド

- **`__repr__`**, **`__str__`** (文字列表現)
- **`__call__`** (呼び出し可能オブジェクト)
- **`__slots__`** (メモリ最適化)
- **`__getitem__`**, **`__setitem__`**, **`__delitem__`** (コンテナプロトコル)
- **`__iter__`**, **`__next__`** (イテレータプロトコル)
- **`__len__`**, **`__contains__`** (コンテナプロトコル)
- **`__hash__`**, **`__eq__`** (ハッシュと等価性)
- **演算子オーバーロード** (`__add__`, `__sub__`, `__mul__` など)

## 組み込み型

- **複素数** (`complex`, `j` サフィックス)
- **Bytes/Bytearray** (`b"..."`, `bytearray`)
- **Frozenset** (`frozenset()`)
- **Decimal** (`decimal.Decimal`)
- **Fraction** (`fractions.Fraction`)
- **Memoryview** (`memoryview`)
- **Slice オブジェクト** (`slice()`)
- **Ellipsis** (`...`)
- **Range as type** (`for` ループ外での `range` オブジェクト)

## 組み込み関数 (ネイティブ変換)

- **リフレクション**: `getattr()`, `setattr()`, `hasattr()`, `delattr()`
- **型チェック**: `issubclass()`, `type()` (※`isinstance` は部分サポート)
- **イントロスペクション**: `dir()`, `vars()`, `locals()`, `globals()`
- **オブジェクト識別**: `id()`, `hash()`
- **イテレーション**: `iter()`, `next()`
- **動的実行**: `exec()`, `eval()`, `compile()`
- **オブジェクト生成**: `object()` (※`super()` はサポート)
- **フォーマット**: `format()`, `repr()`
- **メモリ**: `memoryview()`, `bytearray()`

> [!NOTE]
> これらの多くは Resident Worker 経由で使用可能です。

## 演算子と式



## 標準ライブラリ (ネイティブ変換)

以下は *純粋なRust* には変換できませんが、Resident Worker 経由で動作します:

- **ファイルI/O** (`open()`, ファイル操作) ※`with open(...)` はV1.6.0でネイティブサポート
- **正規表現** (`re` モジュール)
- **日付/時間** (`datetime` モジュール)
- **コレクション** (`collections` モジュール: `deque`, `Counter`, `OrderedDict`)
- **Itertools** (`itertools` モジュール)
- **Functools** (`functools` モジュール: `partial`, `reduce`)
- **モジュールシステム** (相対インポートを含む複雑な複数ファイルプロジェクト)
- **Pickle** (`pickle` モジュール)
- **JSON** (`json` モジュール) - Rust の `serde_json` を使用推奨
- **OS/Sys** (`os`, `sys` モジュール)
- **スレッド/マルチプロセス** (`threading`, `multiprocessing`)
- **ネットワーク** (`socket`, `http`, `urllib`)
- **サブプロセス** (`subprocess` モジュール)

## Resident Worker サポート ✅

IPC 経由で動作するライブラリ (ネイティブ Rust ではない):

- **numpy**, **pandas**, **scipy**, **opencv** (cv2)
- Python 環境の **任意のライブラリ**

### Resident ライブラリでも非サポートの構文

Resident Worker 使用時でも:

- **外部型の型エイリアス**: `NDInt = npt.NDArray[np.int64]`
- **高度な演算子オーバーロード**: `df[df["col"] > 5]` (Pandas フィルタリング)
- **オブジェクト固有メソッド**: 一部のメソッドは型情報が失われる可能性あり

## ノート

ここに記載されている機能は将来のバージョンで追加される可能性があります。機能リクエストについては GitHub リポジトリで Issue を作成してください。
