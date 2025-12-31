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
- **Early Return** handling (refined in v1.2.0)

## Functions

- **Function definitions** with type hints
- **Return statements** with optional values
- **Recursion** support
- **Nested functions** (closure conversion to Rust closures)
- **Lambda expressions** (`lambda x: x + 1`)
- **Higher-order functions** (passing functions as arguments)
- **Named arguments** (`func(name="value")`)
- **Default argument values** (`def func(x=10)`) (V1.2.0)

## Data Structures

- **List literals** and operations
- **List comprehensions** (basic and conditional)
- **Dictionary literals** and operations
- **Tuple literals** and unpacking
- **Struct definitions** (via class syntax)
- **Negative indexing** (`nums[-1]`)
- **Slice notation** (`[:3]`, `[-3:]`, `[1:n-1]`)
- **Index swap** (`a[i], a[j] = a[j], a[i]` → `a.swap()`)
- **List copy** (`l.copy()` → `l.to_vec()`) (V1.2.0)

## Classes & Objects

- **Basic class definitions** with `__init__`
- **Instance attributes** (`self.attr`)
- **Method definitions**
- **Static methods** (`@staticmethod`)
- **Dataclasses** (`@dataclass`) (V1.2.0 Partial)

## Resident Python Worker (V1.2.0) 🆕

Tsuchinoko V1.2.0 introduces a Resident Python Worker to support libraries that are difficult to transpile directly to Rust.

- **NumPy** (`import numpy as np`)
- **Pandas** (`import pandas as pd`)
- **SciPy**
- Any other library accessible in your Python environment via IPC calls.

### Persistent Object Handles 🆕

Tsuchinoko now supports persisting Python objects across bridge calls. This allows for:
- **Complex object state**: Keep DataFrames, NumPy arrays, or custom class instances in memory.
- **Method chaining**: Call multiple methods on the same object handle.
- **Index access**: Direct access to Python object elements via handles (`df["column"]`).
- **Handle integration**: Seamlessly pass handles back to other Python library functions.

## Built-in Functions

- `len()` - get length
- `range()` - numeric range iteration
- `print()` - console output (supports f-string debug `"{x=}"` / `"{:?}"`)
- `list()` - convert to list
- `min()`, `max()` - min/max values
- `abs()` - absolute value
- `int()`, `float()`, `str()`, `bool()` - type conversions

## String Features

- **String literals** (single/double quotes)
- **F-strings** (`f"Hello {name}"`)
  - Debug format `"{x=}"` / `"{:?}"` supported (V1.2.0)
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
- **Type Narrowing** (`if x is None` / `if x is not None`)

## Experimental: PyO3 Direct Integration

> [!NOTE]
> Direct PyO3 calls are still supported but the **Resident Worker** is recommended for compatibility.
