#!/bin/bash
# Tsuchinoko Benchmark Script
# Uses hyperfine to compare Python vs Rust (Tsuchinoko) performance

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Ensure tmp directory exists
mkdir -p "$PROJECT_DIR/tmp"

echo "=== Tsuchinoko Benchmark ==="
echo ""

# --- Fibonacci Benchmark ---
echo "--- Fibonacci (N=35) ---"
echo "This measures recursive function call overhead."
echo ""

BENCH_FILE="$PROJECT_DIR/examples/benchmarks/fibonacci.py"
RUST_FILE="$PROJECT_DIR/tmp/fibonacci.rs"
RUST_BIN="$PROJECT_DIR/tmp/fibonacci"

# Transpile
echo "Transpiling..."
cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- "$BENCH_FILE" -o "$RUST_FILE"

# Compile Rust
echo "Compiling Rust..."
rustc -O "$RUST_FILE" -o "$RUST_BIN"

echo ""
echo "Running benchmark with hyperfine..."
echo ""

hyperfine \
    --warmup 1 \
    --runs 5 \
    --export-markdown "$PROJECT_DIR/tmp/benchmark_result.md" \
    "python3 $BENCH_FILE" \
    "$RUST_BIN"

echo ""
echo "=== Benchmark Complete ==="
echo "Results saved to: tmp/benchmark_result.md"
echo ""

# Show results
cat "$PROJECT_DIR/tmp/benchmark_result.md"
