"""
V1.3.0 演算子テスト

テスト対象:
- @ 行列演算子
- not in 演算子
- ビット演算子: &, |, ^, ~, <<, >>
- ビット累算代入: &=, |=, ^=, <<=, >>=
- **= 累乗代入
"""

# ============================================================
# not in 演算子
# ============================================================

def test_not_in_list(nums: list[int], x: int) -> bool:
    """リストに含まれないかチェック"""
    return x not in nums


def test_not_in_dict(data: dict[str, int], key: str) -> bool:
    """辞書にキーが含まれないかチェック"""
    return key not in data


def test_not_in_string(text: str, sub: str) -> bool:
    """文字列に含まれないかチェック"""
    return sub not in text


# ============================================================
# ビット演算子
# ============================================================

def test_bitwise_and(a: int, b: int) -> int:
    """ビット AND"""
    return a & b


def test_bitwise_or(a: int, b: int) -> int:
    """ビット OR"""
    return a | b


def test_bitwise_xor(a: int, b: int) -> int:
    """ビット XOR"""
    return a ^ b


def test_bitwise_not(a: int) -> int:
    """ビット NOT"""
    return ~a


def test_left_shift(a: int, n: int) -> int:
    """左シフト"""
    return a << n


def test_right_shift(a: int, n: int) -> int:
    """右シフト"""
    return a >> n


# ============================================================
# ビット累算代入演算子
# ============================================================

def test_bitwise_and_assign(a: int, b: int) -> int:
    """ビット AND 累算代入"""
    result: int = a
    result &= b
    return result


def test_bitwise_or_assign(a: int, b: int) -> int:
    """ビット OR 累算代入"""
    result: int = a
    result |= b
    return result


def test_bitwise_xor_assign(a: int, b: int) -> int:
    """ビット XOR 累算代入"""
    result: int = a
    result ^= b
    return result


def test_left_shift_assign(a: int, n: int) -> int:
    """左シフト累算代入"""
    result: int = a
    result <<= n
    return result


def test_right_shift_assign(a: int, n: int) -> int:
    """右シフト累算代入"""
    result: int = a
    result >>= n
    return result


# ============================================================
# **= 累乗代入演算子
# ============================================================

def test_pow_assign(base: int, exp: int) -> int:
    """累乗の累算代入"""
    result: int = base
    result **= exp
    return result


# ============================================================
# @ 行列演算子 (NumPy使用)
# ============================================================
import numpy as np


def test_matrix_mul() -> None:
    """行列乗算 @演算子"""
    a: np.ndarray = np.array([[1, 2], [3, 4]])
    b: np.ndarray = np.array([[5, 6], [7, 8]])
    c: np.ndarray = a @ b
    print(c)


# ============================================================
# メイン
# ============================================================

def main() -> None:
    # not in テスト
    nums: list[int] = [1, 2, 3, 4, 5]
    print(test_not_in_list(nums, 6))  # True
    print(test_not_in_list(nums, 3))  # False

    data: dict[str, int] = {"a": 1, "b": 2}
    print(test_not_in_dict(data, "c"))  # True
    print(test_not_in_dict(data, "a"))  # False

    print(test_not_in_string("hello", "x"))  # True
    print(test_not_in_string("hello", "ll"))  # False

    # ビット演算テスト
    print(test_bitwise_and(0b1100, 0b1010))  # 8 (0b1000)
    print(test_bitwise_or(0b1100, 0b1010))   # 14 (0b1110)
    print(test_bitwise_xor(0b1100, 0b1010))  # 6 (0b0110)
    print(test_bitwise_not(0b1100))          # -13
    print(test_left_shift(1, 4))             # 16
    print(test_right_shift(16, 2))           # 4

    # ビット累算代入テスト
    print(test_bitwise_and_assign(0b1100, 0b1010))  # 8
    print(test_bitwise_or_assign(0b1100, 0b1010))   # 14
    print(test_bitwise_xor_assign(0b1100, 0b1010))  # 6
    print(test_left_shift_assign(1, 4))             # 16
    print(test_right_shift_assign(16, 2))           # 4

    # 累乗代入テスト
    print(test_pow_assign(2, 10))  # 1024

    # 行列演算テスト
    test_matrix_mul()


if __name__ == "__main__":
    main()
