"""
Phase 1 & 2 テスト: not in + ビット演算子
累算代入はローカル変数のみで行う
"""

def test_not_in(nums: list[int], x: int) -> bool:
    return x not in nums


def test_bitwise(a: int, b: int) -> int:
    result: int = a & b
    result = result | 4
    result = result ^ 1
    result = result << 2
    result = result >> 1
    return result


def test_bitwise_not(x: int) -> int:
    return ~x


def test_aug_assign(initial: int) -> int:
    x: int = initial
    x &= 15
    x |= 32
    x ^= 8
    x <<= 1
    x >>= 2
    x **= 2
    return x


def main() -> None:
    nums: list[int] = [1, 2, 3]
    print(test_not_in(nums, 10))
    print(test_not_in(nums, 2))
    
    print(test_bitwise(12, 10))
    print(test_bitwise_not(12))
    print(test_aug_assign(255))


if __name__ == "__main__":
    main()
