# Tsuchinoko VS Code Extension

Transform Python to Rust directly in your editor!

## Features

- **Rust Preview** (`Ctrl+Alt+P` / `Cmd+Alt+P`) - See transpiled code in real-time
- **Status Bar Button** - Click "🚀 Rust Preview" when editing Python files
- **Real-time Diagnostics** - Unsupported syntax highlighted with squiggly lines
- **Auto-check on Save** - Instant feedback on compatibility

## Requirements

- VS Code 1.70+
- Tsuchinoko (`tnk` command) installed and accessible in PATH

## Installation

### From VSIX

```bash
code --install-extension tsuchinoko-0.1.0.vsix
```

### From Source

```bash
cd vscode-extension
npm install
npm run compile
npx vsce package
code --install-extension tsuchinoko-0.1.0.vsix
```

## Usage

1. Open a Python file
2. Press `Ctrl+Alt+P` (or `Cmd+Alt+P` on Mac)
3. A side panel shows the transpiled Rust code

Or click "🚀 Rust Preview" in the status bar (bottom right).

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `tsuchinoko.tnkPath` | `tnk` | Path to the tnk command |
| `tsuchinoko.autoCheck` | `true` | Automatically check on save |
| `tsuchinoko.checkDelay` | `500` | Delay before checking (ms) |

## License

MIT License - Tane Channel Technology
