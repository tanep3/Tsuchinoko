# Supported Features

This document lists all Python features currently supported by Tsuchinoko transpiler.

## Core Syntax

- **Variable declarations** with type hints (`x: int = 10`)
- **Basic types**: `int`, `float`, `str`, `bool`, `None`
- **Collection types**: `list[T]`, `dict[K, V]`, `tuple[...]`
- **Optional types**: `Optional[T]`, `T | None`
- **Optional patterns**: `x or default` → `unwrap_or`, ternary with None check (V1.5.0)
- **Arithmetic operators**: `+`, `-`, `*`, `/`, `//`, `%`, `**`, `@` (V1.3.0)
- **Comparison operators**: `==`, `!=`, `<`, `>`, `<=`, `>=`
- **Chained comparisons** (`0 < x < 10` → `0 < x && x < 10`) (V1.6.0)
- **Logical operators**: `and`, `or`, `not`
- **Membership operators**: `in`, `not in` (V1.3.0)
- **Identity operators**: `is`, `is not` (with `None` comparison)
- **Bitwise operators**: `&`, `|`, `^`, `~`, `<<`, `>>` (V1.3.0)
- **Augmented assignment**: `+=`, `-=`, `*=`, `/=`, `//=`, `%=`, `**=`, `&=`, `|=`, `^=`, `<<=`, `>>=` (V1.3.0)
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
- **\*\*kwargs** (`def func(**kwargs)` → `HashMap<String, Value>`) (V1.6.0)

## Data Structures

- **List literals** and operations
- **List comprehensions** (basic and conditional)
- **Dict comprehensions** (`{k: v for k, v in items}`) (V1.3.0)
- **Dictionary literals** and operations
- **Tuple literals** and unpacking
- **Set literals** (`{1, 2, 3}` → `HashSet`) (V1.5.0)
- **Set comprehensions** (`{x*2 for x in nums}` → `HashSet`) (V1.6.0)
- **Struct definitions** (via class syntax)
- **Negative indexing** (`nums[-1]`)
- **Slice notation** (`[:3]`, `[-3:]`, `[1:n-1]`)
- **Step slicing** (`[::2]`, `[::-1]`) (V1.5.0)
- **Index swap** (`a[i], a[j] = a[j], a[i]` → `a.swap()`)
- **List copy** (`l.copy()` → `l.to_vec()`) (V1.2.0)
- **Multi-assignment** (`a, b, c = 1, 2, 3`) (V1.3.0)
- **List methods**: `pop`, `insert`, `remove`, `extend`, `clear` (V1.5.0)
- **Dict methods**: `keys`, `values`, `get`, `pop`, `update` (V1.5.0)
- **Set methods**: `add`, `remove`, `discard`, `union`, `intersection` (V1.5.0)

## Classes & Objects

- **Basic class definitions** with `__init__`
- **Instance attributes** (`self.attr`)
- **Method definitions**
- **Static methods** (`@staticmethod`)
- **Dataclasses** (`@dataclass`) (V1.2.0 Partial)
- **Single inheritance** (`class Child(Parent)`) → Composition (V1.6.0)
- **super() calls** (`super().method()` → `self.base.method()`) (V1.6.0)
- **@property decorator** → getter/setter methods (V1.6.0)

## Resource Management (V1.6.0)

- **with statement** → RAII scope (`with open(...) as f:` → `{ let f = ...; }`)
- Automatic resource cleanup via Rust's Drop trait

## Resident Python Worker (V1.2.0) 🆕

Tsuchinoko V1.2.0 introduces a Resident Python Worker to support libraries that are difficult to transpile directly to Rust.

- **NumPy** (`import numpy as np`)
- **Pandas** (`import pandas as pd`)
- **OpenCV** (`import cv2`) (V1.4.0)
- **SciPy**
- Any other library accessible in your Python environment via IPC calls.

### `from` Import Syntax (V1.4.0) 🆕

- **Direct function import**: `from numpy import mean, std`
- Automatically converted to `py_bridge.call_json("numpy.mean", ...)` calls

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
- `enumerate()` - indexed iteration (V1.3.0)
- `zip()` - parallel iteration (V1.3.0)
- `sorted()` - sorted list generation (V1.3.0)
- `reversed()` - reverse iteration (V1.3.0)
- `sum()` - sum calculation (V1.3.0)
- `all()`, `any()` - boolean check (V1.3.0)
- `map()`, `filter()` - functional transformation (V1.3.0)
- `assert` - assertion statement (V1.3.0)
- `input()` - user input with optional prompt (V1.5.0)
- `round()` - rounding with precision (V1.5.0)
- `chr()`, `ord()` - character/code point conversion (V1.5.0)
- `bin()`, `hex()`, `oct()` - number format conversion (V1.5.0)
- `isinstance()` - type checking → `DynamicValue` enum + `match` (V1.6.0)

## Math Module (V1.3.0 / V1.4.0)

- **Functions**: `math.sqrt`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `exp`, `log`, `log10`, `log2`, `abs`, `floor`, `ceil`, `round`
- **Constants (V1.4.0)**: `math.pi`, `math.e`, `math.tau`, `math.inf`, `math.nan` → Native Rust constants

## String Features

- **String literals** (single/double quotes)
- **F-strings** (`f"Hello {name}"`)
  - Debug format `"{x=}"` / `"{:?}"` supported (V1.2.0)
- **String methods**: `.upper()`, `.lower()`, `.strip()`, `.split()`, `.join()`, etc.
- **String methods (V1.5.0)**: `.replace()`, `.startswith()`, `.endswith()`, `.find()`, `.rfind()`, `.index()`, `.count()`
- **String predicates (V1.5.0)**: `.isdigit()`, `.isalpha()`, `.isalnum()`

## Error Handling

- **try/except** blocks (converted to `catch_unwind`)
- **try/except with multiple exception types** (`except (ValueError, TypeError):`) (V1.5.0)
- **except with variable** (`except ValueError as e:`) (V1.5.0)
- **try/except/finally** blocks (V1.5.0)
- **try/except/else** blocks (`else` runs when no exception) (V1.5.2)
- **raise** statements (converted to `Err(TsuchinokoError)` or `panic!`)
- **raise from** (`raise A from B`) - exception chaining with `cause` (V1.5.2)
- **Result type unification** - exception functions return `Result<T, TsuchinokoError>` (V1.5.2)
- **Error line numbers** - Python source line included in error messages (V1.5.2)
- **ValueError**, **TypeError** (converted to `TsuchinokoError`)

## Type System

- **Type aliases** (`MyType = list[int]`)
- **Callable types** (`Callable[[T], U]`)
- **Function type inference**
- **Automatic type coercion** (Auto-Ref, Auto-Deref, Auto-Clone)
- **Type Narrowing** (`if x is None` / `if x is not None`)

## Experimental: PyO3 Direct Integration

> [!NOTE]
> Direct PyO3 calls are still supported but the **Resident Worker** is recommended for compatibility.
