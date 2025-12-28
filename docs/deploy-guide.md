# Tsuchinoko Deploy Guide

## Table of Contents

1. [Development Setup](#development-setup)
2. [Building for Release](#building-for-release)
3. [Distribution](#distribution)
4. [CI/CD Integration](#cicd-integration)

---

## Development Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

### Clone and Build

```bash
git clone https://github.com/tanep3/Tsuchinoko.git
cd Tsuchinoko
cargo build
```

### Run Tests

```bash
cargo test
```

---

## Building for Release

### Optimized Build

```bash
cargo build --release
```

Binary location: `target/release/tnk`

### Install Locally

```bash
cargo install --path .
```

This installs `tnk` to `~/.cargo/bin/`.

---

## Distribution

### Cross-Platform Builds

For cross-compilation, use `cross`:

```bash
cargo install cross

# Linux
cross build --release --target x86_64-unknown-linux-gnu

# macOS
cross build --release --target x86_64-apple-darwin

# Windows
cross build --release --target x86_64-pc-windows-gnu
```

### Binary Size Optimization

Add to `Cargo.toml`:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
```

---

## CI/CD Integration

### GitHub Actions Example

`.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test

  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
      - uses: actions/upload-artifact@v4
        with:
          name: tnk-linux
          path: target/release/tnk
```

### Release Workflow

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
      - uses: softprops/action-gh-release@v1
        with:
          files: target/release/tnk
```

---

## Version Management

Update version in:

1. `Cargo.toml` - `version = "x.y.z"`
2. `src/main.rs` - `#[command(version = "x.y.z")]`

Create release:

```bash
git tag v0.5.0
git push origin v0.5.0
```
