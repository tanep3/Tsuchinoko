# Tsuchinoko User Manual

## Table of Contents

1. [Quick Start](#quick-start)
2. [Installation](#installation)
3. [Basic Usage](#basic-usage)
4. [Command Line Options](#command-line-options)
5. [Writing Compatible Python](#writing-compatible-python)
6. [Common Patterns](#common-patterns)
7. [Type Hints Reference](#type-hints-reference)
8. [Limitations](#limitations)

---

## Quick Start

> **Time Required**: 5 minutes

Create `hello.py`:

```python
def greet(name: str) -> str:
    return f"Hello, {name}!"

def main():
    message: str = greet("Tsuchinoko")
    print(message)

main()
```

Transpile and run:

```bash
tnk hello.py -o hello.rs
rustc hello.rs -o hello
./hello
```

Output:
```
Hello, Tsuchinoko!
```

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

For scripts using external libraries (NumPy, Pandas, etc.):

```bash
# Activate venv first
source venv/bin/activate

tnk your_file.py --project my_project
cd my_project
cargo run --release
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

## Common Patterns

### Lists

```python
nums: list[int] = [1, 2, 3, 4, 5]
doubled: list[int] = [x * 2 for x in nums]
nums.append(6)
first: int = nums.pop(0)
```

### Dictionaries

```python
scores: dict[str, int] = {"Alice": 90, "Bob": 85}
alice_score: int = scores["Alice"]
scores["Charlie"] = 88
for key in scores.keys():
    print(key)
```

### Sets (v1.5.0)

```python
s: set[int] = {1, 2, 3}
s.add(4)
s.remove(1)
union: set[int] = s | {5, 6}
```

### Optional Values

```python
from typing import Optional

def find(items: list[int], target: int) -> Optional[int]:
    for i, item in enumerate(items):
        if item == target:
            return i
    return None

result: Optional[int] = find([1, 2, 3], 2)
value: int = result or -1  # x or default pattern
```

### Slicing (v1.5.0)

```python
nums: list[int] = [0, 1, 2, 3, 4, 5]
first_three: list[int] = nums[:3]
reversed_nums: list[int] = nums[::-1]
every_other: list[int] = nums[::2]
```

### Exception Handling (v1.5.0)

```python
try:
    result: int = int("abc")
except ValueError as e:
    print("Invalid input")
finally:
    print("Cleanup")
```

---

## Type Hints Reference

| Python Type | Rust Type |
|-------------|-----------|
| `int` | `i64` |
| `float` | `f64` |
| `str` | `String` |
| `bool` | `bool` |
| `list[T]` | `Vec<T>` |
| `dict[K, V]` | `HashMap<K, V>` |
| `set[T]` | `HashSet<T>` |
| `tuple[T, U]` | `(T, U)` |
| `Optional[T]` | `Option<T>` |
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

- `**kwargs` (keyword arguments)
- Complex class inheritance
- Generators and `yield`
- `async`/`await`
- Custom exception classes
- `raise ... from ...` (planned for v1.5.1)
- `try/except/else` (planned for v1.5.1)

### Edge Cases

- Empty lists with `max()` will panic (same as Python)
- Negative numbers in radix sort are not handled

---

## Examples

See the `examples/` directory for working examples:

- `examples/simple/` - Basic transpilation examples (54 files)
- `examples/import/` - External library examples (8 files)
- `examples/benchmarks/` - Performance benchmarks

---

## See Also

- [Supported Features](supported_features.md)
- [Unsupported Features](unsupported_features.md)
