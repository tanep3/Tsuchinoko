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

## Standard Library / External Libraries

> [!NOTE]
> Many standard and external libraries are now supported via the **Resident Python Worker (V1.2.0)**.
> However, native Rust transpilation is not available for everything.

### Not Supported (Native Transpilation)

These features cannot be transpiled to *pure Rust* yet, but may work via Resident Worker:

- **File I/O** (`open()`, file operations)
- **Regular expressions** (`re` module)
- **Date/Time** (`datetime` module)
- **Collections** (`collections` module beyond dict)
- **Module system** (Complex multi-file projects with relative imports)

### Supported via Resident Worker ✅

Previously unsupported libraries that now work via IPC:

- **numpy**
- **pandas**
- **scipy**
- **scipy**
- **ctypes**-dependent libraries

### Partial / Not Supported Syntax in Resident Libraries

Even if a library is supported via Resident Worker, some Python syntax cannot be transpiled:

- **Type Aliases with External Types**: `NDInt = npt.NDArray[np.int64]` (Cannot resolve external types)
- **Advanced Operator Overloading**: `df[df["col"] > 5]` (Pandas filtering syntax is complex to transpile)
- **Object-specific Methods**: Some methods returning complex types might lose type information (`-> _`).

## Advanced Features

- **Set comprehensions** (`{x for x in ...}`)
- **Global/Nonlocal** statements

## Notes

Features listed here may be added in future versions. For feature requests, please open an issue on the GitHub repository.
