# Tsuchinoko VS Code Extension

Transform Python to Rust directly in your editor!

## Features

- **Rust Preview** (`Ctrl+Alt+P` / `Cmd+Alt+P`) - See transpiled code in real-time
- **Status Bar Button** - Click "🚀 Rust Preview" when editing Python files
- **Accurate Diagnostics** (V0.2.0) - Precise error markers using `--diag-json`
  - **Column-accurate squiggly lines** - Highlights exact error locations
  - **Diagnostic codes** - Shows error codes (e.g., `TNK-UNSUPPORTED-SYNTAX`)
  - **Severity mapping** - Error/Warning/Info levels
- **On-Demand Check** (V0.2.0) - Diagnostics run **only when you preview**, not on save

## Requirements

- VS Code 1.70+
- Tsuchinoko (`tnk` command) installed and accessible in PATH

## Installation

### From VSIX

```bash
code --install-extension tsuchinoko-0.2.0.vsix
```

### From Source

```bash
cd vscode-extension
npm install
npm run compile
npx vsce package
code --install-extension tsuchinoko-0.2.0.vsix
```

## Usage

1. Open a Python file
2. Press `Ctrl+Alt+P` (or `Cmd+Alt+P` on Mac)
3. A side panel shows the transpiled Rust code
4. Diagnostics are displayed if transpilation fails

Or click "🚀 Rust Preview" in the status bar (bottom right).

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `tsuchinoko.tnkPath` | `tnk` | Path to the tnk command |

> **Note**: `autoCheck` and `checkDelay` settings have been removed in v0.2.0.  
> Diagnostics now run only when you execute the preview command.

## License

MIT License - Tane Channel Technology
