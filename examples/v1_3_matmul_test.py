"""
V1.3.0 @演算子テスト (NumPy行列乗算)
"""
import numpy as np


def test_matmul() -> None:
    """@演算子による行列乗算"""
    a = np.array([[1, 2], [3, 4]])
    b = np.array([[5, 6], [7, 8]])
    result = a @ b
    print(result)
    # 期待値: [[19 22], [43 50]]


def main() -> None:
    test_matmul()


if __name__ == "__main__":
    main()
