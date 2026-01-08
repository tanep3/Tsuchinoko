# 変更履歴 (Changelog)

本プロジェクトの主要な変更点をここに記録します。

## [1.5.2] - 2026-01-08 - Result型エラーハンドリング

### 追加 - 例外チェイン (`raise from`)

- **`raise A from B`**: 例外チェーン (`TsuchinokoError.cause` で原因保持)
- **エラーチェーン表示**: `Caused by:` 形式でエラーメッセージ表示
- **行番号追跡**: Python ソース行番号をエラーに含む (`[line 10] RuntimeError: ...`)

### 追加 - `try/except/else` ブロック

- **`else` ブロック**: 例外が発生しなかった場合のみ実行
- **複数例外型**: `except (ValueError, TypeError):`
- **例外変数**: `except ValueError as e:`

### 追加 - Result型統一

- **3層エラーハンドリング・アーキテクチャ**:
  1. **Result統一**: `raise` → `Err(TsuchinokoError)`、`?` で伝播
  2. **外部境界**: PyO3/py_bridge 失敗 → `Err(TsuchinokoError)` (panic しない)
  3. **catch_unwind 診断**: 想定外panic → `InternalError`
- **2パス may_raise 解析**: may_raise 関数を呼ぶ関数は自動的に昇格
- **呼び出し側からの List 型推論**: `def f(nums: list)` + `f([1,2,3])` → `&[i64]`

### 追加 - Hoisting 修正

- **for ループ変数 Hoisting**: `_loop_` プレフィックスでシャドーイング回避
- **Hoisted 変数への代入**: ループ開始時に `i = Some(_loop_i);`

### 変更

- **Python 構文カバレッジ**: 68% → **71%** (78機能サポート)

### テスト

- **リグレッションテスト**: 72/72 パス (100%)
- **新規テスト**: v1.5.2 システムテスト 10件追加

## [1.5.1 - VS Code 拡張 0.1.0] - 2026-01-06

### 追加

- **Rust プレビュー** (`Ctrl+Alt+P` / `Cmd+Alt+P`) - リアルタイムで変換結果を表示
- **ステータスバーボタン** - Python ファイル編集時に「🚀 Rust Preview」をクリック
- **リアルタイム診断** - 保存時に非対応構文を波線でハイライト
- **import 自動検出** - import を含むファイルは自動的に `--project` モードを使用
- **設定可能** - `tsuchinoko.tnkPath`, `tsuchinoko.autoCheck`, `tsuchinoko.checkDelay`

### 技術情報

- ローカルとリモート (WSL/SSH) 両方の環境で動作
- OS の一時ディレクトリに一時ファイルを配置し、activate/deactivate 時に自動クリーンアップ
- 外部ライブラリ (NumPy, Pandas 等) を使用するファイルはプロジェクトベースでプレビュー

## [1.5.0] - 2026-01-05 - 構文網羅的サポート

### 追加 - Set型サポート

- **Setリテラル**: `{1, 2, 3}` → `HashSet::from([1, 2, 3])`
- **Setコンストラクタ**: `set([1, 2, 3])` → `HashSet`
- **Setメソッド**: `add`, `remove`, `discard`, `union`, `intersection`, `difference`
- **Set演算子**: `|` (和集合), `&` (積集合), `-` (差集合)
- **Set包含判定**: `x in s` → `s.contains(&x)`

### 追加 - コレクションメソッド拡張

- **Listメソッド**: `pop()`, `pop(i)`, `insert(i, x)`, `remove(x)`, `extend(iter)`, `clear()`
- **Dictメソッド**: `keys()`, `values()`, `get(k)`, `get(k, default)`, `pop(k)`, `update(other)`

### 追加 - 文字列メソッド拡張

- **置換**: `.replace(old, new)`
- **前後判定**: `.startswith()`, `.endswith()`
- **検索**: `.find()`, `.rfind()`, `.index()`, `.count()`
- **文字種判定**: `.isdigit()`, `.isalpha()`, `.isalnum()`

### 追加 - 組み込み関数

- **ユーザー入力**: `input()`, `input(prompt)`
- **四捨五入**: `round(x)`, `round(x, n)`
- **文字変換**: `chr(n)`, `ord(c)`
- **数値フォーマット**: `bin(x)`, `hex(x)`, `oct(x)`

### 追加 - スライス拡張

- **ステップスライス**: `arr[::2]` → `.iter().step_by(2).cloned().collect()`
- **逆順スライス**: `arr[::-1]` → `.iter().rev().cloned().collect()`
- **範囲+ステップ**: `arr[1:10:2]`

### 追加 - Optional/None 深い対応

- **Union型パース**: `str | None` → `Option<String>`
- **Orパターン**: `x or default` → `x.unwrap_or(default)`
- **三項演算子+None**: `x if x is not None else y` → `if x.is_some() { x.unwrap() } else { y }`
- **自動Someラップ**: Optional型への非None値代入時に自動的に `Some()` でラップ

### 追加 - 例外処理強化

- **複数例外型**: `except (ValueError, TypeError):`
- **例外変数**: `except ValueError as e:`
- **finallyブロック**: `try/except/finally`

### 変更

- **Python構文カバレッジ**: **68%** (75機能サポート)

### テスト

- **リグレッションテスト**: 62/62 パス (100%)
- **新規テスト**: v1.5.0 システムテスト 7件追加
  - `v1_5_set_test.py`, `v1_5_list_methods_test.py`, `v1_5_dict_methods_test.py`
  - `v1_5_string_methods_test.py`, `v1_5_builtins_test.py`, `v1_5_slice_test.py`
  - `v1_5_optional_test.py`

---

## [1.4.0] - 2026-01-04 - 外部ライブラリ機能強化

### 追加 - 外部ライブラリ

- **`from module import func` 構文**: 関数の直接インポートをサポート
  - `from numpy import mean, std` → `py_bridge.call_json("numpy.mean", ...)`
  - PythonBridge 呼び出しへ自動変換
- **外部ライブラリ自動検出**: ネイティブでないモジュールを自動検出
  - `numpy`/`pandas` のハードコード判定を削除
  - すべての外部インポートが Resident Worker を使用
- **`--project` オプション強制**: 外部ライブラリ使用時にエラーメッセージ表示
  - `--project` を使用した適切な依存関係設定を案内
- **OpenCV 対応**: `cv2` をテスト済み外部ライブラリに追加

### 追加 - Math モジュール

- **ネイティブ定数 (V1.4.0)**: `math.pi`, `math.e`, `math.tau`, `math.inf`, `math.nan`
  - Rust ネイティブ定数に変換 (`std::f64::consts::PI` など)
  - ゼロオーバーヘッド - Rust 定数として直接コンパイル

### 変更

- **`pyo3_imports` → `external_imports`**: 内部フィールド名を明確化
- **Python 構文カバレッジ**: 62% → **73%** (78機能サポート)

### テスト

- **リグレッションテスト**: 54/54 パス (100%)
- **新規テスト**: `v1_4_math_constants_test.py`, `v1_4_from_import_test.py`, `v1_4_opencv_simple.py`

---

## [1.3.3] - 2026-01-04 - テストモジュール化 & リグレッション修正

### 変更 - コード品質

- **大規模テストモジュール化**: テストを `src/*/tests/` サブディレクトリに抽出
  - `semantic/mod.rs`: 5,088行 → 820行 (**84%削減**)
  - `parser/mod.rs`: 3,331行 → 2,223行 (**33%削減**)
  - `emitter/mod.rs`: 3,775行 → 1,618行 (**57%削減**)
  - **合計: メインモジュールから7,537行削減**
- **TDD 準拠**: テストが `tests/mod.rs` サブディレクトリ内で実装と同一場所に配置
- **examples フォルダ分割**: `examples/simple/` (45件) と `examples/import/` (6件)
- **リグレッションテストスクリプト**: `run_regression_tests.py` を import テスト用に `--project` 対応

### 修正

- **テスト分散スクリプト**: v1.3.2 で 40件のテストが消失していた問題を修正
- **負のインデックス リグレッション**: Cast でラップされた `arr[-1]` 処理を修正
  - `extract_negative_index` ヘルパー関数を追加
  - `arr[(-1i64 as usize)]` ではなく `arr[arr.len() - 1]` を正しく生成

### テスト

- **ユニットテスト**: 809件パス
- **リグレッションテスト**: 51/51 パス (100%)

## [1.3.2] - 2026-01-04 テスト分散・TDD改善

### 変更 - コード品質

- **semantic モジュールテスト分散**:
  - 48件の式関連テストを `analyze_expressions.rs` に移動
  - 42件の文関連テストを `analyze_statements.rs` に移動
  - `mod.rs` を 6,242行 → 4,819行に削減 (23%削減)
  - TDDワークフロー改善: テストが実装と同一ファイルに配置

### 修正

- **CI/CD Lintエラー**: 重複 `#[test]` 属性 45件を削除
- **Clippy警告**: `useless_conversion`, `len_zero`, `non_snake_case` を修正

### テスト

- **合計**: ユニットテスト 769件 + 統合テスト 7件
- **リグレッション**: 51/51 サンプル成功

## [1.3.1] - 2026-01-02 コードベースリファクタリング

### 変更 - アーキテクチャ

- **semantic モジュール分割**:
  - `type_infer.rs`: 型推論ロジック (TypeInferenceトレイト)
  - `operators.rs`: 演算子変換ロジック
  - `coercion.rs`: 型変換・強制判定
  - `builtins.rs`: 組み込み関数テーブル駆動管理
  - `analyze_statements.rs`: 文解析 (for, while, if, class等)
  - `analyze_expressions.rs`: 式解析 (binop, unary, list, dict等)
  - `analyze_calls.rs`: 関数/メソッド呼び出し解析
  - `analyze_types.rs`: 型ヒント・型解決


- **IR モジュール分割**:
  - `ops.rs`: 演算子定義 (IrBinOp, IrUnaryOp, IrAugAssignOp)
  - `exprs.rs`: 式定義 (IrExpr)
  - `nodes.rs`: ステートメント定義 (IrNode)

- **bridge/strategies 追加**:
  - ImportStrategy トレイト: インポート方式の抽象化
  - NativeStrategy: Rustネイティブ実装 (math系)
  - PyO3Strategy: 将来のPyO3直接呼び出し用 (空箱)
  - ResidentStrategy: 常駐プロセスフォールバック

### 変更 - 責務分離

- `IrExpr::Cast`: int/float型キャストをsemantic側で生成
- `IrExpr::StructConstruct`: struct構築をsemantic側で判定
- emitter側の重複コード26行削減

### テスト - カバレッジ大幅向上

- **semantic モジュール**: 21% → **62%** (+41%向上)
- **全体カバレッジ**: 55% → **66.58%** (+11.6%向上)
- **テスト数**: 465件 → **854件** (+389件追加)
- emitter: **82%** / parser: **80%** 維持

## [1.3.0] - 2026-01-01 基本構文徹底サポート

### 追加 - 演算子

- **`@` 行列演算子**: NumPy 行列乗算に対応
- **`not in` 演算子**: コンテナ非包含チェックに対応
- **ビット演算子**: `&`, `|`, `^`, `~`, `<<`, `>>` に対応
- **ビット累算代入**: `&=`, `|=`, `^=`, `<<=`, `>>=` に対応
- **`**=` 累乗代入**: 累乗の累算代入に対応

### 追加 - 組み込み関数

- **`enumerate`**: インデックス付きイテレーションに対応
- **`zip`**: 複数イテラブルの並列イテレーションに対応
- **`sorted`**: ソート済みリスト生成に対応
- **`reversed`**: 逆順イテレーションに対応
- **`sum`**: 合計計算に対応
- **`all` / `any`**: 全要素/任意要素の真偽判定に対応
- **`map` / `filter`**: 関数型イテレータに対応
- **`assert`**: アサーション文に対応

### 追加 - リストメソッド

- **`.sort()`**: インプレースソートに対応
- **`.reverse()`**: インプレース逆順に対応
- **`.index()`**: 要素の位置検索に対応
- **`.count()`**: 要素の出現回数カウントに対応

### 追加 - その他

- **辞書内包表記**: `{k: v for k, v in items}` に対応
- **多重代入の強化**: `a, b, c = 1, 2, 3` の完全サポート

## [1.2.0] - 2025-12-31

### 追加 (Added)
- **常駐プロセス方式 (Resident Python Worker)**: NumPy や Pandas などの複雑なライブラリを、IPC 経由で常駐プロセスとして呼び出す新アーキテクチャを追加。これにより PyO3 のバイナリ互換性問題を解決。
- **Dataclass 対応**: `@dataclass` デコレータの部分的なサポートを追加。
- **リストのコピー**: `list.copy()` メソッドに対応（`.to_vec()` に変換）。
- **デフォルト引数**: 関数定義におけるデフォルト引数に対応。
- **F-string デバッグ**: f-string 内での `"{:?}"` フォーマット指定に対応。

### 変更 (Changed)
- **型推論**: 変数代入や戻り値の型推論精度を向上。
- **アーリーリターン**: 関数内の早期リターン処理ロジックを改善。
- **インポート戦略**: 最適な変換戦略を選択するハイブリッド方式 (Native > Resident > PyO3) を導入。

## [1.1.0] - 2025-12-29

### 変更 (Changed)
- **演算子の改善**: `is` / `is not` 演算子を修正し、`Option` 型を正しく扱えるように改善。
- **ドキュメント**: ドキュメント構成を見直し、英語版 (`README.md`) と日本語版 (`README_jp.md`) に分離。

### 追加 (Added)
- **機能ドキュメント**: `supported_features.md` および `unsupported_features.md` を追加。
