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

## PyO3 / External Libraries

> [!IMPORTANT]
> PyO3 integration is experimental.

- **numpy >= 2.0** (use numpy < 2.0 due to rust-numpy/ctypes constraints)
- **Complex PyO3 operations** (direct C API calls may fail)
- **Awaiting PyO3/rust-numpy updates for numpy 2.x support**

## Notes

Features listed here may be added in future versions. For feature requests, please open an issue on the GitHub repository.
