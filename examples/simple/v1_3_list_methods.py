"""
V1.3.0 リストメソッドテスト

テスト対象:
- .sort()
- .reverse()
- .index()
- .count()
"""

# ============================================================
# .sort() インプレースソート
# ============================================================

def test_sort_basic(nums: list[int]) -> list[int]:
    """基本的なsort"""
    result: list[int] = list(nums)  # コピー
    result.sort()
    return result


def test_sort_reverse(nums: list[int]) -> list[int]:
    """逆順sort"""
    result: list[int] = list(nums)
    result.sort(reverse=True)
    return result


def test_sort_key(words: list[str]) -> list[str]:
    """key関数付きsort"""
    result: list[str] = list(words)
    result.sort(key=lambda x: len(x))
    return result


# ============================================================
# .reverse() インプレース逆順
# ============================================================

def test_reverse_basic(nums: list[int]) -> list[int]:
    """基本的なreverse"""
    result: list[int] = list(nums)
    result.reverse()
    return result


def test_reverse_strings(words: list[str]) -> list[str]:
    """文字列リストのreverse"""
    result: list[str] = list(words)
    result.reverse()
    return result


# ============================================================
# .index() 要素の位置検索
# ============================================================

def test_index_basic(nums: list[int], x: int) -> int:
    """基本的なindex"""
    return nums.index(x)


def test_index_string(words: list[str], target: str) -> int:
    """文字列リストでのindex"""
    return words.index(target)


# ============================================================
# .count() 要素の出現回数カウント
# ============================================================

def test_count_basic(nums: list[int], x: int) -> int:
    """基本的なcount"""
    return nums.count(x)


def test_count_string(chars: list[str], target: str) -> int:
    """文字列リストでのcount"""
    return chars.count(target)


# ============================================================
# 複合テスト
# ============================================================

def test_sort_and_reverse(nums: list[int]) -> list[int]:
    """sortしてからreverse"""
    result: list[int] = list(nums)
    result.sort()
    result.reverse()
    return result


def test_find_max_count(nums: list[int]) -> tuple[int, int]:
    """最も出現回数が多い要素を見つける"""
    max_count: int = 0
    max_elem: int = nums[0]
    for x in nums:
        c: int = nums.count(x)
        if c > max_count:
            max_count = c
            max_elem = x
    return max_elem, max_count


# ============================================================
# メイン
# ============================================================

def main() -> None:
    # sort テスト
    unsorted: list[int] = [3, 1, 4, 1, 5, 9, 2, 6]
    print(test_sort_basic(unsorted))   # [1, 1, 2, 3, 4, 5, 6, 9]
    print(test_sort_reverse(unsorted)) # [9, 6, 5, 4, 3, 2, 1, 1]
    words: list[str] = ["banana", "apple", "kiwi", "cherry"]
    print(test_sort_key(words))  # ['kiwi', 'apple', 'banana', 'cherry']

    # reverse テスト
    print(test_reverse_basic([1, 2, 3, 4, 5]))  # [5, 4, 3, 2, 1]
    print(test_reverse_strings(["a", "b", "c"]))  # ['c', 'b', 'a']

    # index テスト
    nums: list[int] = [10, 20, 30, 40, 50]
    print(test_index_basic(nums, 30))  # 2
    fruits: list[str] = ["apple", "banana", "cherry"]
    print(test_index_string(fruits, "banana"))  # 1

    # count テスト
    repeated: list[int] = [1, 2, 2, 3, 3, 3, 4, 4, 4, 4]
    print(test_count_basic(repeated, 3))  # 3
    chars: list[str] = ["a", "b", "a", "c", "a"]
    print(test_count_string(chars, "a"))  # 3

    # 複合テスト
    print(test_sort_and_reverse([5, 2, 8, 1, 9]))  # [9, 8, 5, 2, 1]
    print(test_find_max_count([1, 2, 2, 3, 3, 3, 2, 2]))  # (2, 4)


if __name__ == "__main__":
    main()
