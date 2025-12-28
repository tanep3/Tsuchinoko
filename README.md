# Tsuchinoko 🐍➡️🦀

**Python to Rust Transpiler** - Convert type-hinted Python code to Rust

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-1.0.0-green.svg)](Cargo.toml)

[🇯🇵 日本語版はこちら](README_jp.md)

## Overview

Tsuchinoko is a transpiler that converts type-hinted Python code to Rust.
Write logic in Python's readable syntax and gain Rust's safety and performance.

## Design Philosophy

Tsuchinoko is **not** a general-purpose Python compiler. It is designed to:

- **Preserve human-readable logic**: The generated Rust code should be readable and maintainable.
- **Convert imperative Python into structural Rust**: Maps Python control flow directly to Rust equivalents.
- **Prefer borrowing over ownership**: Automatically uses references (`&[T]`, `&str`) where possible to avoid unnecessary allocations.

## Key Features ✨

- **Core Syntax** - Variables, types, operators, control flow
- **Higher-Order Functions** - Pass functions as arguments, closures
- **Lambda Expressions** - `lambda x: x + 1` → `|x| x + 1`
- **Basic Class Support** - Struct-like classes with `__init__` and methods
- **List Comprehensions** - `[x*2 for x in nums if x > 0]`
- **Smart Type Inference** - Auto-Ref, Auto-Deref, minimal `mut`

## Benchmarks 🚀

Comparison between Python (3.x) and Tsuchinoko-generated Rust code (compiled with `rustc -O`).
Benchmarks include data generation (LCG) and sorting time.

| Algorithm | N | Python | Tsuchinoko (Rust) | Speedup |
|-----------|---|--------|-------------------|---------|
| **Bubble Sort** | 10,000 | 5.050s | **0.040s** | **~125x** 🚀 |
| **Radix Sort** | 10,000,000 | 9.711s | **0.311s** | **~31x** 🚀 |

*Measured using `hyperfine` on local environment (Linux x86_64).*

## Installation

```bash
git clone https://github.com/TaneChannelTechnology/Tsuchinoko.git
cd Tsuchinoko
cargo build --release
cargo install --path .
```

## Usage

```bash
# Basic transpilation
tnk your_file.py

# Specify output
tnk your_file.py -o output.rs

# Generate Cargo project
tnk your_file.py --project my_project

# Check only (no output)
tnk your_file.py --check
```

### Input Example (Python)

```python
def bubble_sort(lists: list[int]) -> tuple[list[int], int]:
    sorted_list: list[int] = list(lists)
    list_length: int = len(sorted_list)
    for i in range(list_length):
        for j in range(list_length - i - 1):
            if sorted_list[j] > sorted_list[j + 1]:
                temp: int = sorted_list[j]
                sorted_list[j] = sorted_list[j + 1]
                sorted_list[j + 1] = temp
    return sorted_list, list_length
```

### Output Example (Rust)

```rust
fn bubble_sort(lists: &[i64]) -> (Vec<i64>, i64) {
    let mut sorted_list: Vec<i64> = lists.to_vec();
    let list_length: i64 = sorted_list.len() as i64;
    for i in 0..list_length {
        for j in 0..((list_length - i) - 1) {
            if sorted_list[j as usize] > sorted_list[(j + 1) as usize] {
                let temp: i64 = sorted_list[j as usize];
                sorted_list[j as usize] = sorted_list[(j + 1) as usize];
                sorted_list[(j + 1) as usize] = temp;
            }
        }
    }
    return (sorted_list, list_length);
}
```

## Feature Documentation

For detailed feature lists, see:

- [Supported Features](docs/supported_features.md) | [サポート機能一覧](docs/supported_features_ja.md)
- [Unsupported Features](docs/unsupported_features.md) | [非サポート機能一覧](docs/unsupported_features_ja.md)

## Future Roadmap

- [ ] Full class inheritance support
- [ ] Exception handling (`try-except` → `Result`)
- [ ] Named arguments support
- [ ] More standard library mappings

## Documentation

- [User Manual](docs/user-manual.md) | [ユーザーマニュアル](docs/user-manual_jp.md)
- [Deploy Guide](docs/deploy-guide.md) | [デプロイガイド](docs/deploy-guide_jp.md)
- [Requirements](docs/requirements.md)
- [System Design](docs/system-design.md)
- [API Spec](docs/api-spec.md)

## Testing

```bash
cargo test
```

## Project Structure

```
src/
├── lib.rs          # Library entry point
├── main.rs         # CLI entry point
├── parser/         # Python parser (pest)
├── semantic/       # Semantic analysis & type inference
├── ir/             # Intermediate representation
├── emitter/        # Rust code generation
└── error/          # Error types
```

## License

MIT License

## Author

**Tane Channel Technology**
