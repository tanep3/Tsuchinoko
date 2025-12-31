#!/usr/bin/env python3
import glob
import subprocess
import os
import sys
import time

def run_test(py_file):
    print(f"Testing {py_file}...", end="", flush=True)
    
    # 1. Transpile
    # ソースファイルパスは絶対パスで指定することを推奨（cargo run の引数解釈のため）
    abs_py_file = os.path.abspath(py_file)
    output_rs = "src/bin/regression_test.rs"
    
    cmd_transpile = ["cargo", "run", "--quiet", "--bin", "tnk", "--", abs_py_file, "-o", output_rs]
    
    try:
        proc = subprocess.run(cmd_transpile, capture_output=True, text=True, check=True)
    except subprocess.CalledProcessError as e:
        print(" ❌ Transpile Failed")
        print(e.stderr)
        return False

    # 2. Run
    cmd_run = ["cargo", "run", "--quiet", "--bin", "regression_test"]
    
    try:
        # タイムアウトを設定 (例: 10秒)
        start_time = time.time()
        proc = subprocess.run(cmd_run, capture_output=True, text=True, check=True, timeout=15)
        duration = time.time() - start_time
        print(f" ✅ OK ({duration:.2f}s)")
        # print(proc.stdout) # 必要なら出力
        return True
    except subprocess.TimeoutExpired:
        print(" ❌ Timeout")
        return False
    except subprocess.CalledProcessError as e:
        print(" ❌ Execution Failed")
        print(e.stderr)
        # エラーの詳細を表示
        print("--- stdout ---")
        print(e.stdout)
        print("--- stderr ---")
        print(e.stderr)
        return False

def main():
    # src/bin ディレクトリを作成
    os.makedirs("src/bin", exist_ok=True)
    
    files = sorted(glob.glob("examples/*.py"))
    
    success_count = 0
    fail_count = 0
    failures = []
    
    # 除外リスト (実行に引数が必要だったり、現状動かないことがわかっているもの)
    excludes = [
        # "examples/bench_radix.py", # 時間がかかるかも
    ]
    
    target_files = [f for f in files if f not in excludes]
    
    print(f"Running regression tests on {len(target_files)} files...")
    
    for f in target_files:
        if run_test(f):
            success_count += 1
        else:
            fail_count += 1
            failures.append(f)
            
    print("-" * 40)
    print(f"Total: {len(target_files)}, Success: {success_count}, Fail: {fail_count}")
    
    if failures:
        print("Failures:")
        for f in failures:
            print(f"  - {f}")
            
    if fail_count > 0:
        sys.exit(1)

if __name__ == "__main__":
    main()
