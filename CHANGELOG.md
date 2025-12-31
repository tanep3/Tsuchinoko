# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
