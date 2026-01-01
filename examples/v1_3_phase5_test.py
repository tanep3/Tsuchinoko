"""
Phase 5 テスト: sorted/reversed/sum (assertなし)
"""

def test_sorted(nums: list[int]) -> list[int]:
    """sorted関数"""
    return sorted(nums)


def test_reversed(nums: list[int]) -> list[int]:
    """reversed関数"""
    result: list[int] = []
    for x in reversed(nums):
        result.append(x)
    return result


def test_sum(nums: list[int]) -> int:
    """sum関数"""
    return sum(nums)


def main() -> None:
    nums: list[int] = [3, 1, 4, 1, 5]
    print(test_sorted(nums))    # [1, 1, 3, 4, 5]
    print(test_reversed(nums))  # [5, 1, 4, 1, 3]
    print(test_sum(nums))       # 14


if __name__ == "__main__":
    main()
