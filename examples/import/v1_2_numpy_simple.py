#!/usr/bin/env python3
"""
v1_2_numpy_simple.py
シンプルな numpy テスト（常駐プロセス方式）
"""

from typing import List
import numpy as np


def test_numpy_mean(values: List[int]) -> float:
    """numpy.mean を使って平均を計算"""
    return np.mean(values)


def main() -> None:
    print("=== NumPy Resident Test ===")
    
    result: float = test_numpy_mean([1, 2, 3, 4, 5])
    print(f"mean([1,2,3,4,5]) = {result}")
    
    print("=== Done ===")


if __name__ == "__main__":
    main()
