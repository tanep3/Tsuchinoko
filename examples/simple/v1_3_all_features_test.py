"""
V1.3.0 全機能統合テスト

全てのV1.3.0新機能を使用した統合テストファイル。
Tsuchinokoで変換後、正常にコンパイル・実行できることを確認する。

テスト対象:
- 演算子: not in, ビット演算, **=
- 組み込み関数: enumerate, zip, sorted, reversed, sum, all, any, map, filter, assert
- リストメソッド: .sort(), .reverse(), .index(), .count()
- その他: 辞書内包表記, 多重代入

注意: @演算子はNumPy依存のため、別ファイル v1_3_operators.py でテスト
"""

# ============================================================
# 演算子テスト
# ============================================================

def demo_not_in() -> None:
    """not in 演算子のデモ"""
    nums: list[int] = [1, 2, 3, 4, 5]
    if 10 not in nums:
        print("10 is not in the list")
    if 3 not in nums:
        print("This should not print")


def demo_bitwise() -> None:
    """ビット演算子のデモ"""
    a: int = 0b1100  # 12
    b: int = 0b1010  # 10

    print(a & b)   # 8  (AND)
    print(a | b)   # 14 (OR)
    print(a ^ b)   # 6  (XOR)
    print(~a)      # -13 (NOT)
    print(a << 2)  # 48 (左シフト)
    print(a >> 2)  # 3  (右シフト)


def demo_aug_assign() -> None:
    """累算代入演算子のデモ"""
    x: int = 8
    x &= 4
    print(x)  # 0

    y: int = 8
    y |= 4
    print(y)  # 12

    z: int = 2
    z **= 10
    print(z)  # 1024


# ============================================================
# 組み込み関数テスト
# ============================================================

def demo_enumerate_zip() -> None:
    """enumerate と zip のデモ"""
    fruits: list[str] = ["apple", "banana", "cherry"]
    prices: list[int] = [100, 200, 150]

    # enumerate
    for i, fruit in enumerate(fruits):
        print(f"{i}: {fruit}")

    # enumerate with start
    for i, fruit in enumerate(fruits, start=1):
        print(f"#{i} {fruit}")

    # zip
    for fruit, price in zip(fruits, prices):
        print(f"{fruit} costs {price}")


def demo_sorted_reversed() -> None:
    """sorted と reversed のデモ"""
    nums: list[int] = [3, 1, 4, 1, 5, 9, 2, 6]

    # sorted
    print(sorted(nums))
    print(sorted(nums, reverse=True))

    # reversed
    for x in reversed(nums):
        print(x)


def demo_sum_all_any() -> None:
    """sum, all, any のデモ"""
    nums: list[int] = [1, 2, 3, 4, 5]
    print(sum(nums))  # 15
    print(sum(nums, 100))  # 115

    print(all(x > 0 for x in nums))  # True
    print(any(x > 10 for x in nums))  # False


def demo_map_filter() -> None:
    """map と filter のデモ"""
    nums: list[int] = [1, 2, 3, 4, 5]

    doubled: list[int] = list(map(lambda x: x * 2, nums))
    print(doubled)  # [2, 4, 6, 8, 10]

    evens: list[int] = list(filter(lambda x: x % 2 == 0, nums))
    print(evens)  # [2, 4]


def demo_assert(x: int) -> int:
    """assert のデモ"""
    assert x > 0, "x must be positive"
    return x * 2


# ============================================================
# リストメソッドテスト
# ============================================================

def demo_list_methods() -> None:
    """リストメソッドのデモ"""
    nums: list[int] = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5]

    # count
    print(nums.count(5))  # 3

    # index
    print(nums.index(9))  # 5

    # sort (インプレース)
    sorted_nums: list[int] = list(nums)
    sorted_nums.sort()
    print(sorted_nums)

    # reverse (インプレース)
    reversed_nums: list[int] = list(nums)
    reversed_nums.reverse()
    print(reversed_nums)


# ============================================================
# 辞書内包表記・多重代入テスト
# ============================================================

def demo_dict_comp() -> None:
    """辞書内包表記のデモ"""
    nums: list[int] = [1, 2, 3, 4, 5]

    # 基本
    squares: dict[int, int] = {x: x * x for x in nums}
    print(squares)  # {1: 1, 2: 4, 3: 9, 4: 16, 5: 25}

    # 条件付き
    even_squares: dict[int, int] = {x: x * x for x in nums if x % 2 == 0}
    print(even_squares)  # {2: 4, 4: 16}


def demo_multi_assign() -> None:
    """多重代入のデモ"""
    # 基本
    a, b, c = 1, 2, 3
    print(a, b, c)

    # スワップ
    x: int = 10
    y: int = 20
    x, y = y, x
    print(x, y)  # 20, 10

    # フィボナッチ (多重代入活用)
    fib_a: int = 0
    fib_b: int = 1
    result: list[int] = []
    for _ in range(10):
        result.append(fib_a)
        fib_a, fib_b = fib_b, fib_a + fib_b
    print(result)


# ============================================================
# メイン
# ============================================================

def main() -> None:
    print("=== V1.3.0 Integration Test ===")

    print("\n--- not in ---")
    demo_not_in()

    print("\n--- bitwise ---")
    demo_bitwise()

    print("\n--- augmented assign ---")
    demo_aug_assign()

    print("\n--- enumerate/zip ---")
    demo_enumerate_zip()

    print("\n--- sorted/reversed ---")
    demo_sorted_reversed()

    print("\n--- sum/all/any ---")
    demo_sum_all_any()

    print("\n--- map/filter ---")
    demo_map_filter()

    print("\n--- assert ---")
    print(demo_assert(5))  # 10

    print("\n--- list methods ---")
    demo_list_methods()

    print("\n--- dict comprehension ---")
    demo_dict_comp()

    print("\n--- multi assign ---")
    demo_multi_assign()

    print("\n=== All tests completed! ===")


if __name__ == "__main__":
    main()
