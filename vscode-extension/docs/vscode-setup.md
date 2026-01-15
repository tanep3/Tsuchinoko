# Tsuchinoko VS Code 拡張 導入マニュアル

[🇯🇵 日本語版](vscode-setup_jp.md)

## Prerequisites

- VS Code 1.70 or later
- Tsuchinoko (`tnk` command) installed and accessible in PATH

## Installation

### Option 1: Install from .vsix file

```bash
code --install-extension tsuchinoko-0.2.0.vsix
```

### Option 2: Build from Source

```bash
cd vscode-extension
npm install
npm run compile
npx vsce package
code --install-extension tsuchinoko-0.2.0.vsix
```

## Usage

### Show Rust Preview

1. Open a Python file (`.py`)
2. Press `Ctrl+Alt+P` (or `Cmd+Alt+P` on Mac)
3. A side panel will show the transpiled Rust code

### Real-time Diagnostics (V0.2.0)

- **Column-accurate squiggly lines** using `--diag-json`
- Diagnostics show **only when you run the preview** (not on save)
- Hover over highlighted code to see error details with diagnostic codes

### Commands

| Command | Keybinding | Description |
|---------|------------|-------------|
| `Tsuchinoko: Show Rust Preview` | `Ctrl+Alt+P` | Open Rust preview panel |
| `Tsuchinoko: Transpile to Rust` | - | Transpile current file |

## Configuration

Open VS Code Settings (`Cmd+,`) and search for "Tsuchinoko":

| Setting | Default | Description |
|---------|---------|-------------|
| `tsuchinoko.tnkPath` | `tnk` | Path to the `tnk` command |

> **Note**: `autoCheck` and `checkDelay` settings have been removed in v0.2.0.

## Troubleshooting

### "tnk command not found"

1. Ensure `tnk` is installed: `tnk --version`
2. If installed but not in PATH, set the full path in settings:
   ```json
   "tsuchinoko.tnkPath": "/path/to/tnk"
   ```

### Preview not updating

- Run the preview command again (`Ctrl+Alt+P`)
- Check the Output panel (`View > Output > Tsuchinoko`) for errors

### Diagnostics not showing

- Diagnostics appear only when running the preview command
- Check if the file has `.py` extension

## Uninstall

```bash
code --uninstall-extension tsuchinoko
```
