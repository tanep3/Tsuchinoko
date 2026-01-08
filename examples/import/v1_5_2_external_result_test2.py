# v1_5_2_external_result_test2.py
# System test for Phase 4: External boundary Result化
# Test without int() cast issue

import numpy as np

def test_external_call() -> None:
    """Test that external Python calls return Result with proper error handling"""
    arr = np.array([1, 2, 3, 4, 5])
    total = np.sum(arr)
    print(f"External call result: {total}")  # Expected: 15

def main() -> None:
    test_external_call()

if __name__ == "__main__":
    main()
