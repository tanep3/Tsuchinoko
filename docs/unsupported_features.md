# Unsupported Features

This document lists Python features NOT currently supported by Tsuchinoko transpiler.

## Language Features

- **`**kwargs`** (keyword variable-length arguments)
- **Decorators** (except `@staticmethod`, `@dataclass`)
- **Generators** (`yield` statements)
- **Async/Await** syntax
- **Context managers** (`with` statements)
- **Multiple inheritance**
- **Metaclasses**
- **Walrus operator** (`:=`)

## Exception Handling

- **try/except/finally** with specific exception types
- **Custom exception classes**
- **Exception chaining** (`raise ... from ...`)

## Class Features

- **Class inheritance** (except basic struct-like classes)
- **Properties** (`@property`)
- **Class methods** (`@classmethod`)
- **`__repr__`, `__str__`** and other magic methods
- **Operator overloading** (`__add__`, `__eq__`, etc.)
- **Abstract base classes**

## Built-in Types

- **Set type** (`set[T]`)
- **Frozenset**
- **Complex numbers**
- **Bytes/Bytearray**
- **Decimal**

## Standard Library

- Most standard library modules are not supported
- **File I/O** (`open()`, file operations)
- **Regular expressions** (`re` module)
- **JSON parsing** (`json` module)
- **Date/Time** (`datetime` module)
- **Collections** (`collections` module beyond dict)

## Advanced Features

- **Multiple assignment** (`a, b, c = 1, 2, 3` - partial support only for index swap)
- **Dictionary comprehensions** (`{k: v for k, v in ...}`)
- **Set comprehensions** (`{x for x in ...}`)
- **Global/Nonlocal** statements
- **Module system** (multi-file projects)

## PyO3 / External Libraries (Not Supported)

> [!IMPORTANT]
> PyO3 integration is experimental. The following libraries do NOT work via PyO3.

| Library | Reason |
|---------|--------|
| **ctypes** | Conflicts with PyO3 `auto-initialize` feature |
| **numpy** | Depends on ctypes |
| **pandas** | Depends on numpy → ctypes |
| **scipy** | Depends on numpy → ctypes |
| **pillow** | Depends on ctypes |

### Root Cause

Python's `ctypes` module and PyO3's `auto-initialize` feature have a known incompatibility
when embedding Python in Rust binaries. This is a limitation of PyO3, not Tsuchinoko.

### Workaround

For numpy/pandas workloads, consider:
- Using pure Rust libraries (`ndarray`, `polars`)
- Building as Python extension module with `maturin` (different architecture)

## Notes

Features listed here may be added in future versions. For feature requests, please open an issue on the GitHub repository.
