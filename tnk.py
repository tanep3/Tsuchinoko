#!/usr/bin/env python3
# tnk（Tsuchinokoコンパイラ実行ファイル）

import argparse
from src.compiler import compile_python_to_rust
import os

def main():
    parser = argparse.ArgumentParser(description="Tsuchinoko Python-to-Rust Compiler")
    parser.add_argument("input", help="Input Python file")
    parser.add_argument("-o", "--output", help="Output rs file (optional)")
    args = parser.parse_args()

    input_path = args.input
    output_path = args.output

    if not output_path:
        base, _ = os.path.splitext(input_path)
        output_path = base + ".rs"

    with open(input_path, "r", encoding="utf-8") as f:
        source = f.read()

    c_code = compile_python_to_rust(source)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(c_code)

    print(f"[OK] Compiled '{input_path}' to '{output_path}'")

if __name__ == "__main__":
    main()
