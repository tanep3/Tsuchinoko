# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [VS Code Extension 0.1.0] - 2026-01-06

### Added

- **Rust Preview** (`Ctrl+Alt+P` / `Cmd+Alt+P`) - See transpiled Rust code in real-time
- **Status Bar Button** - Click "🚀 Rust Preview" when editing Python files
- **Real-time Diagnostics** - Unsupported syntax highlighted with squiggly lines on save
- **Auto Import Detection** - Files with imports automatically use `--project` mode
- **Configurable Settings** - `tsuchinoko.tnkPath`, `tsuchinoko.autoCheck`, `tsuchinoko.checkDelay`

### Technical

- Extension runs in both local and remote (WSL/SSH) environments
- Temporary files in OS temp directory with auto-cleanup on activate/deactivate
- Project-based preview for files with external imports (NumPy, Pandas, etc.)

## [1.5.0] - 2026-01-05 - Comprehensive Syntax Coverage

### Added - Set Type Support

- **Set Literals**: `{1, 2, 3}` → `HashSet::from([1, 2, 3])`
- **Set Constructor**: `set([1, 2, 3])` → `HashSet`
- **Set Methods**: `add`, `remove`, `discard`, `union`, `intersection`, `difference`
- **Set Operators**: `|` (union), `&` (intersection), `-` (difference)
- **Set Membership**: `x in s` → `s.contains(&x)`

### Added - Collection Method Extensions

- **List Methods**: `pop()`, `pop(i)`, `insert(i, x)`, `remove(x)`, `extend(iter)`, `clear()`
- **Dict Methods**: `keys()`, `values()`, `get(k)`, `get(k, default)`, `pop(k)`, `update(other)`

### Added - String Method Extensions

- **Replacement**: `.replace(old, new)`
- **Prefix/Suffix**: `.startswith()`, `.endswith()`
- **Search**: `.find()`, `.rfind()`, `.index()`, `.count()`
- **Predicates**: `.isdigit()`, `.isalpha()`, `.isalnum()`

### Added - Built-in Functions

- **User Input**: `input()`, `input(prompt)`
- **Rounding**: `round(x)`, `round(x, n)`
- **Character Conversion**: `chr(n)`, `ord(c)`
- **Number Formatting**: `bin(x)`, `hex(x)`, `oct(x)`

### Added - Slice Enhancements

- **Step Slicing**: `arr[::2]` → `.iter().step_by(2).cloned().collect()`
- **Reverse Slicing**: `arr[::-1]` → `.iter().rev().cloned().collect()`
- **Range with Step**: `arr[1:10:2]`

### Added - Optional/None Deep Support

- **Union Type Parsing**: `str | None` → `Option<String>`
- **Or Pattern**: `x or default` → `x.unwrap_or(default)`
- **Ternary with None**: `x if x is not None else y` → `if x.is_some() { x.unwrap() } else { y }`
- **Auto Some Wrapping**: Non-None values assigned to Optional types are wrapped in `Some()`

### Added - Exception Handling Enhancements

- **Multiple Exception Types**: `except (ValueError, TypeError):`
- **Exception Variable**: `except ValueError as e:`
- **Finally Block**: `try/except/finally`

### Changed

- **Python Syntax Coverage**: **68%** (75 features supported)

### Tests

- **Regression Tests**: 62/62 passed (100%)
- **New Tests**: 7 new v1.5.0 system tests added
  - `v1_5_set_test.py`, `v1_5_list_methods_test.py`, `v1_5_dict_methods_test.py`
  - `v1_5_string_methods_test.py`, `v1_5_builtins_test.py`, `v1_5_slice_test.py`
  - `v1_5_optional_test.py`

---

## [1.4.0] - 2026-01-04 - External Library Enhancements

### Added - External Libraries

- **`from module import func` Syntax**: Support for direct function imports
  - `from numpy import mean, std` → `py_bridge.call_json("numpy.mean", ...)`
  - Automatic conversion to PythonBridge calls
- **Automatic External Library Detection**: Non-native modules are now automatically detected
  - Removed hardcoded `numpy`/`pandas` checks
  - All external imports trigger Resident Worker usage
- **`--project` Enforcement**: Error message when external libraries are used without `--project` flag
  - Clear guidance on using `--project` for proper dependency setup
- **OpenCV Support**: Added `cv2` to tested external libraries

### Added - Math Module

- **Native Constants (V1.4.0)**: `math.pi`, `math.e`, `math.tau`, `math.inf`, `math.nan`
  - Converted to native Rust constants (`std::f64::consts::PI`, etc.)
  - Zero overhead - compiled directly as Rust constants

### Changed

- **`pyo3_imports` → `external_imports`**: Renamed internal field for clarity
- **Python Syntax Coverage**: 62% → **73%** (78 features supported)

### Tests

- **Regression Tests**: 54/54 passed (100%)
- **New Tests**: `v1_4_math_constants_test.py`, `v1_4_from_import_test.py`, `v1_4_opencv_simple.py`

---

## [1.3.3] - 2026-01-04 - Test Modularization & Regression Fix

- **Unit Tests**: 809 passed
- **Regression Tests**: 51/51 passed (100%)

## [1.3.2] - 2026-01-04 - Test Distribution & TDD Improvement

### Changed - Code Quality

- **Semantic Module Test Distribution**:
  - Moved 48 expression-related tests to `analyze_expressions.rs`
  - Moved 42 statement-related tests to `analyze_statements.rs`
  - `mod.rs` reduced from 6,242 to 4,819 lines (23% reduction)
  - Improved TDD workflow: tests now colocated with implementation

### Fixed

- **CI/CD Lint Errors**: Removed 45 duplicate `#[test]` attributes
- **Clippy Warnings**: Fixed `useless_conversion`, `len_zero`, `non_snake_case`

### Tests

- **Total**: 769 unit tests + 7 integration tests
- **Regression**: 51/51 examples pass

## [1.3.1] - 2026-01-02 - Codebase Refactoring

### Changed - Architecture

- **Semantic Module Split**:
  - `type_infer.rs`: Type inference logic (TypeInference trait)
  - `operators.rs`: Operator conversion logic
  - `coercion.rs`: Type coercion and conversion utilities
  - `builtins.rs`: Built-in function table-driven management
  - `analyze_statements.rs`: Statement analysis (for, while, if, class, etc.)
  - `analyze_expressions.rs`: Expression analysis (binop, unary, list, dict, etc.)
  - `analyze_calls.rs`: Function/method call analysis
  - `analyze_types.rs`: Type hint and type resolution

- **IR Module Split**:
  - `ops.rs`: Operator definitions (IrBinOp, IrUnaryOp, IrAugAssignOp)
  - `exprs.rs`: Expression definitions (IrExpr)
  - `nodes.rs`: Statement definitions (IrNode)

- **bridge/strategies Added**:
  - ImportStrategy trait: Abstraction for import handling
  - NativeStrategy: Rust native implementation (math functions)
  - PyO3Strategy: Placeholder for future PyO3 direct calls
  - ResidentStrategy: Resident process fallback

### Changed - Responsibility Separation

- `IrExpr::Cast`: int/float type casts now generated by semantic analyzer
- `IrExpr::StructConstruct`: Struct construction now determined by semantic analyzer
- Removed 26 lines of duplicate code from emitter

### Tests - Major Coverage Improvement

- **semantic module**: 21% → **62%** (+41% improvement)
- **Overall coverage**: 55% → **66.58%** (+11.6% improvement)
- **Test count**: 465 → **854** (+389 new tests)
- emitter: **82%** / parser: **80%** maintained


## [1.3.0] - 2026-01-01 - Thorough Basic Syntax Support

### Added - Operators

- **`@` Matrix Multiplication**: NumPy matrix multiplication support
- **`not in` Operator**: Container non-membership check
- **Bitwise Operators**: Support for `&`, `|`, `^`, `~`, `<<`, `>>`
- **Bitwise Augmented Assignment**: Support for `&=`, `|=`, `^=`, `<<=`, `>>=`
- **`**=` Power Assignment**: Power augmented assignment

### Added - Built-in Functions

- **`enumerate`**: Indexed iteration support
- **`zip`**: Parallel iteration over multiple iterables
- **`sorted`**: Sorted list generation
- **`reversed`**: Reverse iteration support
- **`sum`**: Sum calculation support
- **`all` / `any`**: All/any element boolean check
- **`map` / `filter`**: Functional iterator transformation
- **`assert`**: Assertion statement support

### Added - List Methods

- **`.sort()`**: In-place sorting
- **`.reverse()`**: In-place reversal
- **`.index()`**: Element position lookup
- **`.count()`**: Element count

### Added - Other

- **Dict Comprehension**: Support for `{k: v for k, v in items}`
- **Enhanced Multi-assignment**: Full support for `a, b, c = 1, 2, 3`

## [1.2.0] - 2025-12-31

### Added
- **Resident Python Worker**: New architecture to handle complex Python imports (e.g., `numpy`, `pandas`) via a persistent background process using IPC. This resolves binary compatibility issues with PyO3.
- **Dataclass Support**: Partial support for `@dataclass` decorator.
- **List Copy**: Support for `list.copy()` method (transpiles to `.to_vec()`).
- **Default Arguments**: Support for default values in function arguments.
- **F-string Debug**: Support for `"{:?}"` format specifier in f-strings.

### Changed
- **Type Inference**: Improved accuracy for variable assignments and return types.
- **Early Return**: Refined logic for handling early returns in functions.
- **Import Strategy**: Now uses a Hybrid approach (Native > Resident > PyO3) to select the best transpilation strategy.

## [1.1.0] - 2025-12-29

### Changed
- **Operator Refinement**: Improved `is` / `is not` operators to correctly handle `Option` types.
- **Documentation**: Restructured documentation into separate English (`README.md`) and Japanese (`README_jp.md`) files.

### Added
- **Feature Documentation**: Added detailed `supported_features.md` and `unsupported_features.md`.
