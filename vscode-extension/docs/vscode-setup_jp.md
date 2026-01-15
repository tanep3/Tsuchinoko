# Tsuchinoko VS Code 拡張 導入マニュアル

[🇺🇸 English Version](vscode-setup.md)

## 前提条件

- VS Code 1.70 以降
- Tsuchinoko (`tnk` コマンド) がインストール済みで PATH に含まれていること

## インストール

### 方法 1: .vsix ファイルからインストール

```bash
code --install-extension tsuchinoko-0.2.0.vsix
```

### 方法 2: ソースからビルド

```bash
cd vscode-extension
npm install
npm run compile
npx vsce package
code --install-extension tsuchinoko-0.2.0.vsix
```

## 使い方

### Rust プレビュー表示

1. Python ファイル (`.py`) を開く
2. `Ctrl+Alt+P` (Mac: `Cmd+Alt+P`) を押す
3. サイドパネルに変換された Rust コードが表示される

### リアルタイム診断 (V0.2.0)

- **カラム単位の波線**: `--diag-json` による正確なハイライト
- 診断は**プレビュー実行時のみ**表示（保存時は実行されません）
- ハイライトされた箇所にカーソルを合わせると診断コード付きエラー詳細が表示されます

### コマンド

| コマンド | キーバインド | 説明 |
|---------|------------|------|
| `Tsuchinoko: Show Rust Preview` | `Ctrl+Alt+P` | Rust プレビューパネルを開く |
| `Tsuchinoko: Transpile to Rust` | - | 現在のファイルをトランスパイル |

## 設定

VS Code 設定 (`Cmd+,`) を開き、「Tsuchinoko」で検索:

| 設定 | デフォルト | 説明 |
|------|----------|------|
| `tsuchinoko.tnkPath` | `tnk` | `tnk` コマンドのパス |

> **注意**: `autoCheck` と `checkDelay` 設定は v0.2.0 で削除されました。

## トラブルシューティング

### "tnk command not found"

1. `tnk` がインストールされているか確認: `tnk --version`
2. インストール済みだが PATH にない場合は、設定でフルパスを指定:
   ```json
   "tsuchinoko.tnkPath": "/path/to/tnk"
   ```

### プレビューが更新されない

- プレビューコマンドを再実行 (`Ctrl+Alt+P`)
- 出力パネル (`表示 > 出力 > Tsuchinoko`) でエラーを確認

### 診断が表示されない

- 診断はプレビューコマンド実行時のみ表示されます
- ファイルの拡張子が `.py` か確認

## アンインストール

```bash
code --uninstall-extension tsuchinoko
```
