# Tsuchinoko VS Code 拡張 導入マニュアル

[🇯🇵 日本語版](vscode-setup_jp.md)

## Prerequisites

- VS Code 1.70 or later
- Tsuchinoko (`tnk` command) installed and accessible in PATH

## Installation

### Option 1: Install from .vsix file

```bash
code --install-extension tsuchinoko-0.1.0.vsix
```

### Option 2: Build from Source

```bash
cd vscode-extension
npm install
npm run compile
npx vsce package
code --install-extension tsuchinoko-0.1.0.vsix
```

## Usage

### Show Rust Preview

1. Open a Python file (`.py`)
2. Press `Cmd+Shift+T` (Mac) or `Ctrl+Shift+T` (Windows/Linux)
3. A side panel will show the transpiled Rust code

### Real-time Diagnostics

- Unsupported syntax will be highlighted with red squiggly lines
- Hover over the highlighted code to see error details
- Diagnostics update automatically when you save the file

### Commands

| Command | Keybinding | Description |
|---------|------------|-------------|
| `Tsuchinoko: Show Rust Preview` | `Cmd+Shift+T` | Open Rust preview panel |
| `Tsuchinoko: Transpile to Rust` | - | Transpile current file |

## Configuration

Open VS Code Settings (`Cmd+,`) and search for "Tsuchinoko":

| Setting | Default | Description |
|---------|---------|-------------|
| `tsuchinoko.tnkPath` | `tnk` | Path to the `tnk` command |
| `tsuchinoko.autoCheck` | `true` | Automatically check on save |
| `tsuchinoko.checkDelay` | `500` | Delay before checking (ms) |

## Troubleshooting

### "tnk command not found"

1. Ensure `tnk` is installed: `tnk --version`
2. If installed but not in PATH, set the full path in settings:
   ```json
   "tsuchinoko.tnkPath": "/path/to/tnk"
   ```

### Preview not updating

- Save the file (`Cmd+S`) to trigger refresh
- Check the Output panel (`View > Output > Tsuchinoko`) for errors

### Diagnostics not showing

- Ensure `tsuchinoko.autoCheck` is enabled
- Check if the file has `.py` extension

## Uninstall

```bash
code --uninstall-extension tsuchinoko
```
