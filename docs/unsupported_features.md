# Unsupported Features

This document lists Python features NOT currently supported by Tsuchinoko transpiler.

## Language Constructs

### Statements

- **`del` statement** (deleting variables or elements)
- **`match` statement** (Python 3.10+ pattern matching)
- **`type` statement** (Python 3.12+ type alias syntax)
- **`global` statement** (declaring global variables)
- **`nonlocal` statement** (nested function variable binding)
- **Walrus operator** (`:=` assignment expression)

### Async/Await

- **`async def`** (coroutine definitions)
- **`await`** expressions
- **`async for`** (asynchronous iteration)
- **`async with`** (asynchronous context managers)

### Generators

- **`yield` statement** (generator functions)
- **`yield from`** (generator delegation)
- **Generator expressions** (`(x for x in items)`)



> [!NOTE]
> List comprehensions, dict comprehensions, and set comprehensions are supported.

### Context Managers

- **Custom context managers** (`__enter__` / `__exit__` protocol)

> [!NOTE]
> Basic `with open(...) as f:` and similar patterns are supported (V1.6.0).

### Arguments

- **`**kwargs`** (keyword variable-length arguments)

> [!NOTE]
> Both `*args` and `**kwargs` are supported (V1.6.0).

## Exception Handling

- **Custom exception classes** (defining your own exception types)

## Class Features

### Inheritance & OOP

- **Multiple inheritance** (more than one base class)
- **Multiple inheritance**
- **Abstract base classes** (`abc` module)
- **Metaclasses**

### Decorators

- **Custom decorators** (except `@staticmethod`, `@dataclass`, `@property`)
- **Class methods** (`@classmethod`)

> [!NOTE]
> Single inheritance, `super()`, and `@property` are supported (V1.6.0).

### Magic Methods

- **`__repr__`**, **`__str__`** (string representation)
- **`__call__`** (callable objects)
- **`__slots__`** (memory optimization)
- **`__getitem__`**, **`__setitem__`**, **`__delitem__`** (container protocol)
- **`__iter__`**, **`__next__`** (iterator protocol)
- **`__len__`**, **`__contains__`** (container protocol)
- **`__hash__`**, **`__eq__`** (hashing and equality)
- **Operator overloading** (`__add__`, `__sub__`, `__mul__`, etc.)

## Built-in Types

- **Complex numbers** (`complex`, `j` suffix)
- **Bytes/Bytearray** (`b"..."`, `bytearray`)
- **Frozenset** (`frozenset()`)
- **Decimal** (`decimal.Decimal`)
- **Fraction** (`fractions.Fraction`)
- **Memoryview** (`memoryview`)
- **Slice objects** (`slice()`)
- **Ellipsis** (`...`)
- **Range as type** (`range` objects used outside `for` loops)

## Built-in Functions (Native Transpilation)

- **Reflection**: `getattr()`, `setattr()`, `hasattr()`, `delattr()`
- **Type checking**: `isinstance()`, `issubclass()`, `type()`
- **Introspection**: `dir()`, `vars()`, `locals()`, `globals()`
- **Object identity**: `id()`, `hash()`
- **Iteration**: `iter()`, `next()`
- **Dynamic execution**: `exec()`, `eval()`, `compile()`
- **Formatting**: `format()`, `repr()`

> [!NOTE]
> `isinstance()` and `super()` are supported (V1.6.0).
- **Memory**: `memoryview()`, `bytearray()`

> [!NOTE]
> Many of these can be used via Resident Worker.

## Operators & Expressions

> [!NOTE]
> Chained comparisons (`a < b < c`) are supported (V1.6.0).

## Standard Library (Native Transpilation)

These modules cannot be transpiled to *pure Rust* but work via Resident Worker:

- **File I/O** (`open()`, file operations)
- **Regular expressions** (`re` module)
- **Date/Time** (`datetime` module)
- **Collections** (`collections` module: `deque`, `Counter`, `OrderedDict`)
- **Itertools** (`itertools` module)
- **Functools** (`functools` module: `partial`, `reduce`)
- **Module system** (complex multi-file projects with relative imports)
- **Pickle** (`pickle` module)
- **JSON** (`json` module) - use Rust's `serde_json` instead
- **OS/Sys** (`os`, `sys` modules)
- **Threading/Multiprocessing** (`threading`, `multiprocessing`)
- **Networking** (`socket`, `http`, `urllib`)
- **Subprocess** (`subprocess` module)

## Resident Worker Support ✅

Libraries that work via IPC (not native Rust):

- **numpy**, **pandas**, **scipy**, **opencv** (cv2)
- **Any library** in your Python environment

### Partial / Not Supported Syntax in Resident Libraries

Even with Resident Worker:

- **Type aliases with external types**: `NDInt = npt.NDArray[np.int64]`
- **Advanced operator overloading**: `df[df["col"] > 5]` (Pandas filtering)
- **Object-specific methods**: Some methods may lose type information

## Notes

Features listed here may be added in future versions. For feature requests, please open an issue on the GitHub repository.
