"""
V1.3.0 組み込み関数テスト

テスト対象:
- enumerate
- zip
- sorted
- reversed
- sum
- all / any
- map / filter
- assert
"""

# ============================================================
# enumerate
# ============================================================

def test_enumerate_basic(items: list[str]) -> list[tuple[int, str]]:
    """基本的なenumerate"""
    result: list[tuple[int, str]] = []
    for i, item in enumerate(items):
        result.append((i, item))
    return result


def test_enumerate_start(items: list[str]) -> list[tuple[int, str]]:
    """start引数付きenumerate"""
    result: list[tuple[int, str]] = []
    for i, item in enumerate(items, start=1):
        result.append((i, item))
    return result


# ============================================================
# zip
# ============================================================

def test_zip_two_lists(a: list[int], b: list[str]) -> list[tuple[int, str]]:
    """2つのリストをzip"""
    result: list[tuple[int, str]] = []
    for x, y in zip(a, b):
        result.append((x, y))
    return result


def test_zip_three_lists(a: list[int], b: list[int], c: list[int]) -> list[tuple[int, int, int]]:
    """3つのリストをzip"""
    result: list[tuple[int, int, int]] = []
    for x, y, z in zip(a, b, c):
        result.append((x, y, z))
    return result


# ============================================================
# sorted
# ============================================================

def test_sorted_basic(nums: list[int]) -> list[int]:
    """基本的なsorted"""
    return sorted(nums)


def test_sorted_reverse(nums: list[int]) -> list[int]:
    """逆順sorted"""
    return sorted(nums, reverse=True)


def test_sorted_key(words: list[str]) -> list[str]:
    """key関数付きsorted"""
    return sorted(words, key=lambda x: len(x))


# ============================================================
# reversed
# ============================================================

def test_reversed_list(nums: list[int]) -> list[int]:
    """リストを逆順に"""
    result: list[int] = []
    for x in reversed(nums):
        result.append(x)
    return result


def test_reversed_string(s: str) -> str:
    """文字列を逆順に"""
    result: str = ""
    for c in reversed(s):
        result += c
    return result


# ============================================================
# sum
# ============================================================

def test_sum_basic(nums: list[int]) -> int:
    """基本的なsum"""
    return sum(nums)


def test_sum_with_start(nums: list[int], start: int) -> int:
    """start引数付きsum"""
    return sum(nums, start)


def test_sum_floats(nums: list[float]) -> float:
    """floatのsum"""
    return sum(nums)


# ============================================================
# all / any
# ============================================================

def test_all_true(flags: list[bool]) -> bool:
    """allが真を返すケース"""
    return all(flags)


def test_any_true(flags: list[bool]) -> bool:
    """anyが真を返すケース"""
    return any(flags)


def test_all_with_condition(nums: list[int]) -> bool:
    """条件式でall"""
    return all(x > 0 for x in nums)


def test_any_with_condition(nums: list[int]) -> bool:
    """条件式でany"""
    return any(x < 0 for x in nums)


# ============================================================
# map / filter
# ============================================================

def test_map_basic(nums: list[int]) -> list[int]:
    """基本的なmap"""
    return list(map(lambda x: x * 2, nums))


def test_filter_basic(nums: list[int]) -> list[int]:
    """基本的なfilter"""
    return list(filter(lambda x: x > 0, nums))


def test_map_filter_chain(nums: list[int]) -> list[int]:
    """mapとfilterの連鎖"""
    doubled: list[int] = list(map(lambda x: x * 2, nums))
    return list(filter(lambda x: x > 10, doubled))


# ============================================================
# assert
# ============================================================

def test_assert_basic(x: int) -> int:
    """基本的なassert"""
    assert x > 0
    return x * 2


def test_assert_with_message(x: int) -> int:
    """メッセージ付きassert"""
    assert x >= 0, "x must be non-negative"
    return x + 1


# ============================================================
# メイン
# ============================================================

def main() -> None:
    # enumerate テスト
    items: list[str] = ["a", "b", "c"]
    print(test_enumerate_basic(items))  # [(0, 'a'), (1, 'b'), (2, 'c')]
    print(test_enumerate_start(items))  # [(1, 'a'), (2, 'b'), (3, 'c')]

    # zip テスト
    nums: list[int] = [1, 2, 3]
    strs: list[str] = ["one", "two", "three"]
    print(test_zip_two_lists(nums, strs))  # [(1, 'one'), (2, 'two'), (3, 'three')]
    print(test_zip_three_lists([1, 2], [3, 4], [5, 6]))  # [(1, 3, 5), (2, 4, 6)]

    # sorted テスト
    unsorted: list[int] = [3, 1, 4, 1, 5, 9, 2, 6]
    print(test_sorted_basic(unsorted))   # [1, 1, 2, 3, 4, 5, 6, 9]
    print(test_sorted_reverse(unsorted)) # [9, 6, 5, 4, 3, 2, 1, 1]
    words: list[str] = ["banana", "apple", "kiwi", "cherry"]
    print(test_sorted_key(words))  # ['kiwi', 'apple', 'banana', 'cherry']

    # reversed テスト
    print(test_reversed_list([1, 2, 3, 4, 5]))  # [5, 4, 3, 2, 1]
    print(test_reversed_string("hello"))        # "olleh"

    # sum テスト
    print(test_sum_basic([1, 2, 3, 4, 5]))       # 15
    print(test_sum_with_start([1, 2, 3], 10))   # 16
    print(test_sum_floats([1.5, 2.5, 3.0]))     # 7.0

    # all/any テスト
    print(test_all_true([True, True, True]))   # True
    print(test_all_true([True, False, True]))  # False
    print(test_any_true([False, False, True])) # True
    print(test_any_true([False, False, False]))# False
    print(test_all_with_condition([1, 2, 3]))  # True
    print(test_any_with_condition([1, -2, 3])) # True

    # map/filter テスト
    print(test_map_basic([1, 2, 3, 4, 5]))     # [2, 4, 6, 8, 10]
    print(test_filter_basic([-1, 2, -3, 4]))    # [2, 4]
    print(test_map_filter_chain([1, 2, 3, 4, 5, 6]))  # [12]

    # assert テスト
    print(test_assert_basic(5))  # 10
    print(test_assert_with_message(0))  # 1


if __name__ == "__main__":
    main()
