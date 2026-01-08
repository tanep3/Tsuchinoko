# v1_5_2_external_result_test.py
# System test for Phase 4: External boundary Result化
# PyO3/py_bridge failures should return Err(TsuchinokoError) instead of panic

import numpy as np

def test_external_call() -> int:
    """Test that external Python calls work correctly"""
    arr = np.array([1, 2, 3, 4, 5])
    total = np.sum(arr)
    return int(total)

def main() -> None:
    result = test_external_call()
    print(f"External call result: {result}")  # Expected: 15

if __name__ == "__main__":
    main()
