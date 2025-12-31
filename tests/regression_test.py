import os
import subprocess
import sys
import shutil

EXAMPLES_DIR = "examples"
PROJECT_DIR = "tmp_regression"
TNK_BIN = "target/debug/tnk"

def run_command(cmd, cwd=None):
    try:
        result = subprocess.run(cmd, shell=True, check=True, capture_output=True, text=True, cwd=cwd)
        return True, result.stdout
    except subprocess.CalledProcessError as e:
        return False, e.stderr + e.stdout

def main():
    if not os.path.exists(TNK_BIN):
        print(f"Error: {TNK_BIN} not found. Run 'cargo build' first.")
        sys.exit(1)

    examples = [f for f in os.listdir(EXAMPLES_DIR) if f.endswith(".py")]
    examples.sort()

    passed = []
    failed = []

    for ex in examples:
        print(f"Testing {ex}...", end=" ", flush=True)
        ex_path = os.path.join(EXAMPLES_DIR, ex)
        
        # Cleanup
        if os.path.exists(PROJECT_DIR):
            shutil.rmtree(PROJECT_DIR)
        
        # Transpile
        ok, err = run_command(f"{TNK_BIN} {ex_path} --project {PROJECT_DIR}")
        if not ok:
            print("TRANSPILE FAILED")
            failed.append((ex, "transpile", err))
            continue
        
        # Build
        ok, err = run_command("cargo build", cwd=PROJECT_DIR)
        if not ok:
            print("BUILD FAILED")
            failed.append((ex, "build", err))
            continue
        
        print("OK")
        passed.append(ex)

    print("\n--- Regression Result ---")
    print(f"Total: {len(examples)}")
    print(f"Passed: {len(passed)}")
    print(f"Failed: {len(failed)}")

    if failed:
        print("\nFailures:")
        for ex, stage, err in failed:
            print(f"[{ex}] {stage} error:\n{err[:500]}...\n")
        sys.exit(1)
    else:
        print("All regression tests passed!")
        sys.exit(0)

if __name__ == "__main__":
    main()
