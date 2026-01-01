"""
Phase 4 テスト: リストメソッド
"""

def test_sort(nums: list[int]) -> list[int]:
    """インプレースsort"""
    result: list[int] = []
    for x in nums:
        result.append(x)
    result.sort()
    return result


def test_reverse(nums: list[int]) -> list[int]:
    """インプレースreverse"""
    result: list[int] = []
    for x in nums:
        result.append(x)
    result.reverse()
    return result


def test_index(nums: list[int], x: int) -> int:
    """要素の位置を取得"""
    return nums.index(x)


def test_count(nums: list[int], x: int) -> int:
    """要素の出現回数を取得"""
    return nums.count(x)


def main() -> None:
    nums: list[int] = [3, 1, 4, 1, 5, 9, 2, 6]
    print(test_sort(nums))      # [1, 1, 2, 3, 4, 5, 6, 9]
    print(test_reverse(nums))   # [6, 2, 9, 5, 1, 4, 1, 3]
    print(test_index(nums, 4))  # 2
    print(test_count(nums, 1))  # 2


if __name__ == "__main__":
    main()
