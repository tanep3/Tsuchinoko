# Tsuchinoko VS Code 拡張 導入マニュアル

[🇺🇸 English Version](vscode-setup.md)

## 前提条件

- VS Code 1.70 以降
- Tsuchinoko (`tnk` コマンド) がインストール済みで PATH に含まれていること

## インストール

### 方法 1: .vsix ファイルからインストール

```bash
code --install-extension tsuchinoko-0.1.0.vsix
```

### 方法 2: ソースからビルド

```bash
cd vscode-extension
npm install
npm run compile
npx vsce package
code --install-extension tsuchinoko-0.1.0.vsix
```

## 使い方

### Rust プレビュー表示

1. Python ファイル (`.py`) を開く
2. `Ctrl+Alt+P` (Mac: `Cmd+Alt+P`) を押す
3. サイドパネルに変換された Rust コードが表示される

### リアルタイム診断

- 非対応の構文は赤い波線でハイライトされます
- ハイライトされた箇所にカーソルを合わせるとエラー詳細が表示されます
- 診断はファイル保存時に自動的に更新されます

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
| `tsuchinoko.autoCheck` | `true` | 保存時に自動チェック |
| `tsuchinoko.checkDelay` | `500` | チェック実行までの遅延 (ms) |

## トラブルシューティング

### "tnk command not found"

1. `tnk` がインストールされているか確認: `tnk --version`
2. インストール済みだが PATH にない場合は、設定でフルパスを指定:
   ```json
   "tsuchinoko.tnkPath": "/path/to/tnk"
   ```

### プレビューが更新されない

- ファイルを保存 (`Cmd+S`) して更新をトリガー
- 出力パネル (`表示 > 出力 > Tsuchinoko`) でエラーを確認

### 診断が表示されない

- `tsuchinoko.autoCheck` が有効か確認
- ファイルの拡張子が `.py` か確認

## アンインストール

```bash
code --uninstall-extension tsuchinoko
```
