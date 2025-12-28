# Tsuchinoko デプロイガイド

## 目次

1. [開発環境セットアップ](#開発環境セットアップ)
2. [リリースビルド](#リリースビルド)
3. [配布](#配布)
4. [CI/CD連携](#cicd連携)

---

## 開発環境セットアップ

### 前提条件

```bash
# Rustのインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# インストール確認
rustc --version
cargo --version
```

### クローンとビルド

```bash
git clone https://github.com/tanep3/Tsuchinoko.git
cd Tsuchinoko
cargo build
```

### テスト実行

```bash
cargo test
```

---

## リリースビルド

### 最適化ビルド

```bash
cargo build --release
```

バイナリの場所: `target/release/tnk`

### ローカルインストール

```bash
cargo install --path .
```

これにより `tnk` が `~/.cargo/bin/` にインストールされます。

---

## 配布

### クロスプラットフォームビルド

クロスコンパイルには `cross` を使用します：

```bash
cargo install cross

# Linux
cross build --release --target x86_64-unknown-linux-gnu

# macOS
cross build --release --target x86_64-apple-darwin

# Windows
cross build --release --target x86_64-pc-windows-gnu
```

### バイナリサイズ最適化

`Cargo.toml` に追加：

```toml
[profile.release]
opt-level = "z"     # サイズ最適化
lto = true          # リンク時最適化
codegen-units = 1   # より良い最適化
strip = true        # シンボル削除
```

---

## CI/CD連携

### GitHub Actions サンプル

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

### リリースワークフロー

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

## バージョン管理

以下のファイルでバージョンを更新：

1. `Cargo.toml` - `version = "x.y.z"`
2. `src/main.rs` - `#[command(version = "x.y.z")]`

リリースの作成：

```bash
git tag v0.5.0
git push origin v0.5.0
```
