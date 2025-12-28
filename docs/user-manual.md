# Tsuchinoko User Manual

## Table of Contents

1. [Installation](#installation)
2. [Basic Usage](#basic-usage)
3. [Command Line Options](#command-line-options)
4. [Writing Compatible Python](#writing-compatible-python)
5. [Type Hints Reference](#type-hints-reference)
6. [Limitations](#limitations)

---

## Installation

### Prerequisites

- Rust 1.70 or later
- Cargo (comes with Rust)

### Install from Source

```bash
git clone https://github.com/tanep3/Tsuchinoko.git
cd Tsuchinoko
cargo build --release
cargo install --path .
```

After installation, `tnk` command will be available globally.

---

## Basic Usage

### Single File Transpilation

```bash
tnk your_file.py
```

Output: `your_file.rs` in current directory

### Specify Output Path

```bash
tnk your_file.py -o custom_output.rs
```

### Generate Cargo Project

```bash
tnk your_file.py --project my_project
```

This creates a complete Cargo project structure:
```
my_project/
├── Cargo.toml
├── .gitignore
└── src/
    └── main.rs
```

### Check Mode

```bash
tnk your_file.py --check
```

Validates the Python code without generating output.

---

## Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--output` | `-o` | Specify output file path |
| `--project` | `-p` | Generate Cargo project |
| `--check` | `-c` | Check only, no output |
| `--debug` | `-d` | Show debug information |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

---

## Writing Compatible Python

### Required: Type Hints

All variables and function signatures **must** have type hints:

```python
# ✅ Good
x: int = 10
def add(a: int, b: int) -> int:
    return a + b

# ❌ Bad (no type hints)
x = 10
def add(a, b):
    return a + b
```

### Entry Point

Use the standard Python entry point pattern:

```python
def main() -> None:
    # your code here
    pass

if __name__ == "__main__":
    main()
```

This will generate a proper Rust `main()` function.

---

## Type Hints Reference

| Python Type | Rust Type |
|-------------|-----------|
| `int` | `i64` |
| `float` | `f64` |
| `str` | `String` |
| `bool` | `bool` |
| `list[T]` | `Vec<T>` |
| `tuple[T, U]` | `(T, U)` |
| `None` | `()` |

### Function Parameters

List parameters are automatically passed by reference:

```python
def process(data: list[int]) -> int:  # data becomes &[i64]
    return len(data)
```

---

## Limitations

### Not Supported

- `Optional` types
- Slicing (`arr[1:3]`, `arr[-1]`)
- `break` / `continue`
- Exception handling (`try`/`except`)
- Classes and OOP
- Generators and `yield`
- `async`/`await`

### Edge Cases

- Empty lists with `max()` will panic (same as Python)
- Negative numbers in radix sort are not handled

---

## Examples

See the `examples/` directory for working examples:

- `bubbleSort.py` - Bubble sort implementation
- `recursiveRadixSort.py` - Radix sort implementation
