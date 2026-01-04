#!/usr/bin/env python3
"""
Tsuchinoko Regression Test Runner
=================================

examples/simple/  -> -o オプションで単一ファイルトランスパイル＆実行
examples/import/  -> --project オプションでプロジェクト生成＆ビルド＆実行
"""
import glob
import subprocess
import os
import sys
import time
import shutil

def run_simple_test(py_file):
    """単一ファイルテスト: -o オプションで直接トランスパイル＆実行"""
    print(f"Testing {py_file}...", end="", flush=True)
    
    abs_py_file = os.path.abspath(py_file)
    output_rs = "src/bin/regression_test.rs"
    
    # 1. Transpile
    cmd_transpile = ["cargo", "run", "--quiet", "--bin", "tnk", "--", abs_py_file, "-o", output_rs]
    
    try:
        subprocess.run(cmd_transpile, capture_output=True, text=True, check=True)
    except subprocess.CalledProcessError as e:
        print(" ❌ Transpile Failed")
        print(e.stderr)
        return False

    # 2. Run
    cmd_run = ["cargo", "run", "--quiet", "--bin", "regression_test"]
    
    try:
        start_time = time.time()
        subprocess.run(cmd_run, capture_output=True, text=True, check=True, timeout=15)
        duration = time.time() - start_time
        print(f" ✅ OK ({duration:.2f}s)")
        return True
    except subprocess.TimeoutExpired:
        print(" ❌ Timeout")
        return False
    except subprocess.CalledProcessError as e:
        print(" ❌ Execution Failed")
        print(e.stderr)
        return False


def run_import_test(py_file):
    """Import系テスト: --project オプションでプロジェクト生成＆ビルド＆実行"""
    print(f"Testing {py_file}...", end="", flush=True)
    
    abs_py_file = os.path.abspath(py_file)
    project_dir = "/tmp/tsuchinoko_regression_project"
    
    # プロジェクトディレクトリをクリア
    if os.path.exists(project_dir):
        shutil.rmtree(project_dir)
    
    # 1. Generate project
    cmd_project = ["cargo", "run", "--quiet", "--bin", "tnk", "--", abs_py_file, "--project", project_dir]
    
    try:
        subprocess.run(cmd_project, capture_output=True, text=True, check=True)
    except subprocess.CalledProcessError as e:
        print(" ❌ Project Generation Failed")
        print(e.stderr)
        return False

    # 2. Build
    cmd_build = ["cargo", "build", "--release"]
    
    try:
        subprocess.run(cmd_build, capture_output=True, text=True, check=True, cwd=project_dir, timeout=120)
    except subprocess.TimeoutExpired:
        print(" ❌ Build Timeout")
        return False
    except subprocess.CalledProcessError as e:
        print(" ❌ Build Failed")
        print(e.stderr)
        return False

    # 3. Run
    # プロジェクト名はディレクトリ名から取得
    project_name = os.path.basename(project_dir).replace("-", "_")
    binary_path = os.path.join(project_dir, "target", "release", project_name)
    
    try:
        start_time = time.time()
        subprocess.run([binary_path], capture_output=True, text=True, check=True, timeout=30)
        duration = time.time() - start_time
        print(f" ✅ OK ({duration:.2f}s)")
        return True
    except subprocess.TimeoutExpired:
        print(" ❌ Execution Timeout")
        return False
    except subprocess.CalledProcessError as e:
        print(" ❌ Execution Failed")
        print(e.stderr)
        return False


def main():
    # src/bin ディレクトリを作成
    os.makedirs("src/bin", exist_ok=True)
    
    # 単一テストファイルを取得
    simple_files = sorted(glob.glob("examples/simple/*.py"))
    import_files = sorted(glob.glob("examples/import/*.py"))
    
    success_count = 0
    fail_count = 0
    failures = []
    
    total_tests = len(simple_files) + len(import_files)
    print(f"Running regression tests: {len(simple_files)} simple + {len(import_files)} import = {total_tests} total")
    print()
    
    # Simple tests
    print("=== Simple Tests ===")
    for f in simple_files:
        if run_simple_test(f):
            success_count += 1
        else:
            fail_count += 1
            failures.append(f)
    
    # Import tests
    print()
    print("=== Import Tests (--project) ===")
    for f in import_files:
        if run_import_test(f):
            success_count += 1
        else:
            fail_count += 1
            failures.append(f)
            
    print("-" * 40)
    print(f"Total: {total_tests}, Success: {success_count}, Fail: {fail_count}")
    
    if failures:
        print("Failures:")
        for f in failures:
            print(f"  - {f}")
            
    if fail_count > 0:
        sys.exit(1)

if __name__ == "__main__":
    main()
