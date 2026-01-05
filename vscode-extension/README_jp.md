# Tsuchinoko VS Code 拡張

エディタ内で直接 Python を Rust に変換！

## 機能

- **Rust プレビュー** (`Ctrl+Alt+P` / `Cmd+Alt+P`) - リアルタイムで変換結果を表示
- **ステータスバーボタン** - Python ファイル編集時に「🚀 Rust Preview」をクリック
- **リアルタイム診断** - 非対応構文を波線でハイライト
- **保存時自動チェック** - 互換性について即座にフィードバック

## 必要環境

- VS Code 1.70 以降
- Tsuchinoko (`tnk` コマンド) がインストール済みで PATH に含まれていること

## インストール

### VSIX からインストール

```bash
code --install-extension tsuchinoko-0.1.0.vsix
```

### ソースからビルド

```bash
cd vscode-extension
npm install
npm run compile
npx vsce package
code --install-extension tsuchinoko-0.1.0.vsix
```

## 使い方

1. Python ファイルを開く
2. `Ctrl+Alt+P` (Mac: `Cmd+Alt+P`) を押す
3. サイドパネルに変換された Rust コードが表示される

またはステータスバー（右下）の「🚀 Rust Preview」をクリック。

## 設定

| 設定 | デフォルト | 説明 |
|------|----------|------|
| `tsuchinoko.tnkPath` | `tnk` | tnk コマンドのパス |
| `tsuchinoko.autoCheck` | `true` | 保存時に自動チェック |
| `tsuchinoko.checkDelay` | `500` | チェック実行までの遅延 (ms) |

## ライセンス

MIT License - Tane Channel Technology
