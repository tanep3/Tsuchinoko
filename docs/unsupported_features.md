# Unsupported Features

This document lists Python features NOT currently supported by Tsuchinoko transpiler.

## Language Features

- **Variable-length arguments** (`*args`, `**kwargs`)
- **Decorators** (except `@staticmethod`)
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
- **Starred expressions** (`first, *rest = [1, 2, 3]`)
- **Dictionary comprehensions** (`{k: v for k, v in ...}`)
- **Set comprehensions** (`{x for x in ...}`)
- **Global/Nonlocal** statements
- **Import statements**
- **Module system**

## Notes

Features listed here may be added in future versions. For feature requests, please open an issue on the GitHub repository.
