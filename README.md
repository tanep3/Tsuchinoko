# Tsuchinoko 🐍➡️🦀

**Python to Rust Transpiler** - Convert type-hinted Python code to Rust

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[🇯🇵 日本語版はこちら](README_jp.md)

## Overview

Tsuchinoko is a transpiler that converts type-hinted Python code to Rust.
Write logic in Python's readable syntax and gain Rust's safety and performance.

## Design Philosophy

Tsuchinoko is **not** a general-purpose Python compiler. It is designed to:

- **Preserve human-readable logic**: The generated Rust code should be readable and maintainable.
- **Convert imperative Python into structural Rust**: Maps Python control flow directly to Rust equivalents.
- **Prefer borrowing over ownership**: Automatically uses references (`&[T]`, `&str`) where possible to avoid unnecessary allocations.

## Features

- ✅ **Type hints**: `int`, `str`, `list[int]`, `tuple[int, str]`, `dict[str, int]`, `Optional[int]`
- ✅ **Slice types**: Generates `&[T]` instead of `&Vec<T>` (idiomatic Rust)
- ✅ **Ownership inference**: Automatic reference/ownership decision
- ✅ **Minimal mut**: Only uses `mut` when reassignment is detected
- ✅ **snake_case conversion**: `getOrder` → `get_order` automatic
- ✅ **Rust Optimization**: Maps `dict` to `HashMap`, handles `None` as `Option::None`

## Benchmarks 🚀

Comparison between Python (3.x) and Tsuchinoko-generated Rust code (compiled with `rustc -O`).
Benchmarks include data generation (LCG) and sorting time.

| Algorithm | N | Python | Tsuchinoko (Rust) | Speedup |
|-----------|---|--------|-------------------|---------|
| **Bubble Sort** | 30,000 | 50.313s | **0.436s** | **~115x** 🚀 |
| **Radix Sort** | 10,000,000 | 9.480s | **0.402s** | **~24x** 🚀 |

*Measured on local environment (Linux x86_64).*

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

## Supported Features

| Python Syntax | Rust Output | Status |
|--------------|-------------|--------|
| `x: int = 10` | `let x: i64 = 10;` | ✅ |
| `list[int]` | `Vec<i64>` / `&[i64]` | ✅ |
| `def func(x: int) -> int` | `fn func(x: i64) -> i64` | ✅ |
| `for i in range(n)` | `for i in 0..n` | ✅ |
| `if/elif/else` | `if/else if/else` | ✅ |
| `while` | `while` | ✅ |
| `list(x)` | `x.to_vec()` | ✅ |
| `len(x)` | `x.len()` | ✅ |
| `max(x)` | `x.iter().max().cloned().unwrap()` | ✅ |
| `x ** 2` | `x.pow(2)` | ✅ |
| `x.append(y)` | `x.push(y)` | ✅ |
| `x.extend(y)` | `x.extend(y)` | ✅ |
| `dict[k, v]` | `HashMap<K, V>` | ✅ |
| `x in d` | `d.contains_key(&x)` | ✅ |
| `arr[-1]` | `arr[arr.len()-1]` | ✅ |
| `Optional[T]` | `Option<T>` | ✅ |

## Limitations / Unsupported Features

Tsuchinoko intentionally does **not** support the full Python spec.

- ❌ **Classes & OOP**: No class support (STRUCT-based design is planned).
- ❌ **Exceptions**: No `try-except` (Rust `Result` mapping is planned).
- ❌ **Dynamic Typing**: All variables must have type hints.
- ❌ **Async/Await**: Not supported.
- ❌ **Standard Library**: Most Python standard libraries are not available.
- ❌ **Generators/Yield**: Not supported.
- ❌ **List Comprehensions**: Only basic forms provided (no nested loops/conditionals).
- ❌ **Global Variables**: Global mutable state is discouraged/unsupported.

## Future Roadmap

- [ ] **Benchmarks**: Performance comparison between Python, Tsuchinoko-Rust, and handwritten Rust.
- [ ] **Structs**: Mapping Python classes to Rust structs (Data Classes).
- [ ] **Error Handling**: `try-except` mapping to `Result`.

## Documentation

- [User Manual](docs/user-manual.md)
- [Deploy Guide](docs/deploy-guide.md)
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
