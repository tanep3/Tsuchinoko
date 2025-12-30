# v1_2_pyo3_simple.py - シンプルな PyO3 テスト
# numpy の基本的な配列操作のみ

import numpy as np


def main() -> None:
    print("=== PyO3 Simple Test ===")
    
    # 基本的な配列作成
    arr = np.array([1, 2, 3, 4, 5])
    print(arr)
    
    # mean計算
    mean_val = np.mean(arr)
    print(f"mean = {mean_val}")
    
    print("=== Done ===")


if __name__ == "__main__":
    main()
