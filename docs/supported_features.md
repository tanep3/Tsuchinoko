# Supported Features

This document lists all Python features currently supported by Tsuchinoko transpiler.

## Core Syntax

- **Variable declarations** with type hints (`x: int = 10`)
- **Basic types**: `int`, `float`, `str`, `bool`, `None`
- **Collection types**: `list[T]`, `dict[K, V]`, `tuple[...]`
- **Optional types**: `Optional[T]`, `T | None`
- **Arithmetic operators**: `+`, `-`, `*`, `/`, `//`, `%`, `**`
- **Comparison operators**: `==`, `!=`, `<`, `>`, `<=`, `>=`
- **Logical operators**: `and`, `or`, `not`
- **Identity operators**: `is`, `is not` (with `None` comparison)
- **Augmented assignment**: `+=`, `-=`, `*=`, `/=`, `//=`, `%=`
- **Docstrings**: Triple-quoted strings converted to Rust comments

## Control Flow

- **If/elif/else** statements
- **For loops** with `range()`, collection iteration
- **While loops**
- **Break/Continue** statements
- **Conditional expressions** (`x if cond else y`)

## Functions

- **Function definitions** with type hints
- **Return statements** with optional values
- **Recursion** support
- **Nested functions** (closure conversion to Rust closures)
- **Lambda expressions** (`lambda x: x + 1`)
- **Higher-order functions** (passing functions as arguments)
- **Named arguments** (`func(name="value")`)
- **Default argument values** (`def func(x=10)`)

## Data Structures

- **List literals** and operations
- **List comprehensions** (basic and conditional)
- **Dictionary literals** and operations
- **Tuple literals** and unpacking
- **Struct definitions** (via class syntax)
- **Negative indexing** (`nums[-1]`)
- **Slice notation** (`[:3]`, `[-3:]`, `[1:n-1]`)
- **Index swap** (`a[i], a[j] = a[j], a[i]` → `a.swap()`)

## Classes & Objects

- **Basic class definitions** with `__init__`
- **Instance attributes** (`self.attr`)
- **Method definitions**
- **Static methods** (`@staticmethod`)

## Built-in Functions

- `len()` - get length
- `range()` - numeric range iteration
- `print()` - console output
- `list()` - convert to list
- `min()`, `max()` - min/max values
- `abs()` - absolute value
- `int()`, `float()`, `str()`, `bool()` - type conversions

## String Features

- **String literals** (single/double quotes)
- **F-strings** (`f"Hello {name}"`)
- **String methods**: `.upper()`, `.lower()`, `.strip()`, `.split()`, `.join()`, etc.

## Error Handling

- **try/except** blocks (converted to `catch_unwind`)
- **raise** statements (converted to `panic!`)
- **ValueError**, **TypeError** (converted to `panic!`)

## Type System

- **Type aliases** (`MyType = list[int]`)
- **Callable types** (`Callable[[T], U]`)
- **Function type inference**
- **Automatic type coercion** (Auto-Ref, Auto-Deref, Auto-Clone)

## V1.2.0 New Features

- **`@dataclass`** decorator (basic support)
- **Star unpacking** (`head, *tail = values`)
- **Star args** (`def func(*args)`)
- **Argument spread** (`func(*list)`)
- **Import statements** parsing
- **Type narrowing** (`if x is None` / `if x is not None`)

## PyO3 Integration (Experimental)

> [!WARNING]
> PyO3 integration is experimental. Compatibility depends on your environment.

- **`tnk -p project`** generates Cargo project with PyO3 dependency
- **`--pyo3-version`** option to specify PyO3 version
- **venv must be activated** before running generated binary

### Supported External Libraries (via PyO3)

| Library | Status | Notes |
|---------|--------|-------|
| json | ✅ OK | JSON parsing/serialization |
| math | ✅ OK | Math functions |
| re | ✅ OK | Regular expressions |
| datetime | ✅ OK | Date/time handling |
| os | ✅ OK | OS information |

### Unsupported External Libraries (via PyO3)

| Library | Status | Reason |
|---------|--------|--------|
| ctypes | ❌ NG | Conflicts with PyO3 auto-initialize |
| numpy | ❌ NG | Depends on ctypes |
| pandas | ❌ NG | Depends on numpy → ctypes |

> [!NOTE]
> Libraries that use Python's `ctypes` module cannot work with PyO3's `auto-initialize` feature.
> This is a known limitation of embedding Python in Rust binaries.
