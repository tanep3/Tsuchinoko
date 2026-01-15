# Tsuchinoko VS Code 拡張機能

エディタ内で直接PythonをRustに変換！

## 機能

- **Rust プレビュー** (`Ctrl+Alt+P` / `Cmd+Alt+P`) - リアルタイムで変換結果を表示
- **ステータスバーボタン** - Python ファイル編集時に「🚀 Rust Preview」をクリック
- **正確な診断表示** (V0.2.0) - `--diag-json` を使用した精密なエラーマーカー
  - **カラム単位の波線** - 正確なエラー位置をハイライト
  - **診断コード表示** - エラーコード（例: `TNK-UNSUPPORTED-SYNTAX`）を表示
  - **重要度マッピング** - Error/Warning/Info レベル
- **オンデマンドチェック** (V0.2.0) - プレビュー実行時**のみ**診断を実行（保存時は実行しない）

## 要件

- VS Code 1.70+
- Tsuchinoko (`tnk` コマンド) がインストールされ、PATH に含まれていること

## インストール

### VSIX からインストール

```bash
code --install-extension tsuchinoko-0.2.0.vsix
```

### ソースからビルド

```bash
cd vscode-extension
npm install
npm run compile
npx vsce package
code --install-extension tsuchinoko-0.2.0.vsix
```

## 使い方

1. Python ファイルを開く
2. `Ctrl+Alt+P` (Mac では `Cmd+Alt+P`) を押す
3. サイドパネルに変換された Rust コードが表示される
4. 変換に失敗した場合は診断情報が表示される

または、ステータスバー（右下）の「🚀 Rust Preview」をクリック。

## 設定

| 設定項目 | デフォルト | 説明 |
|---------|----------|------|
| `tsuchinoko.tnkPath` | `tnk` | tnk コマンドのパス |

> **注意**: `autoCheck` と `checkDelay` 設定は v0.2.0 で削除されました。  
> 診断はプレビューコマンド実行時のみ実行されます。

## ライセンス

MIT License - Tane Channel Technology
