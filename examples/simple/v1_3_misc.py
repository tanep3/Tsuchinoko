"""
V1.3.0 その他の構文テスト

テスト対象:
- 辞書内包表記
- 多重代入の強化
"""

# ============================================================
# 辞書内包表記
# ============================================================

def test_dict_comp_basic(keys: list[str], values: list[int]) -> dict[str, int]:
    """基本的な辞書内包表記"""
    return {k: v for k, v in zip(keys, values)}


def test_dict_comp_transform(nums: list[int]) -> dict[int, int]:
    """変換を含む辞書内包表記"""
    return {x: x * x for x in nums}


def test_dict_comp_condition(nums: list[int]) -> dict[int, int]:
    """条件付き辞書内包表記"""
    return {x: x * x for x in nums if x > 0}


def test_dict_comp_enumerate(items: list[str]) -> dict[int, str]:
    """enumerateを使った辞書内包表記"""
    return {i: item for i, item in enumerate(items)}


# ============================================================
# 多重代入
# ============================================================

def test_multi_assign_basic() -> tuple[int, int, int]:
    """基本的な多重代入"""
    a, b, c = 1, 2, 3
    return a, b, c


def test_multi_assign_swap() -> tuple[int, int]:
    """スワップ"""
    a: int = 10
    b: int = 20
    a, b = b, a
    return a, b


def test_multi_assign_from_tuple() -> tuple[int, int, int]:
    """タプルからの代入"""
    t: tuple[int, int, int] = (100, 200, 300)
    x, y, z = t
    return x, y, z


def test_multi_assign_from_list() -> tuple[int, int, int]:
    """リストからの代入"""
    lst: list[int] = [1, 2, 3]
    a, b, c = lst
    return a, b, c


def test_multi_assign_nested() -> tuple[int, int, int, int]:
    """ネストした代入"""
    a, b = 1, 2
    c, d = 3, 4
    return a, b, c, d


# ============================================================
# 複合テスト
# ============================================================

def test_dict_comp_with_multi_assign() -> dict[str, int]:
    """辞書内包表記と多重代入の組み合わせ"""
    pairs: list[tuple[str, int]] = [("a", 1), ("b", 2), ("c", 3)]
    return {k: v * 2 for k, v in pairs}


def fibonacci_multi(n: int) -> list[int]:
    """多重代入を使ったフィボナッチ"""
    result: list[int] = []
    a, b = 0, 1
    for _ in range(n):
        result.append(a)
        a, b = b, a + b
    return result


# ============================================================
# メイン
# ============================================================

def main() -> None:
    # 辞書内包表記テスト
    keys: list[str] = ["a", "b", "c"]
    values: list[int] = [1, 2, 3]
    print(test_dict_comp_basic(keys, values))  # {'a': 1, 'b': 2, 'c': 3}

    nums: list[int] = [1, 2, 3, 4, 5]
    print(test_dict_comp_transform(nums))  # {1: 1, 2: 4, 3: 9, 4: 16, 5: 25}

    mixed: list[int] = [-2, -1, 0, 1, 2]
    print(test_dict_comp_condition(mixed))  # {1: 1, 2: 4}

    items: list[str] = ["apple", "banana", "cherry"]
    print(test_dict_comp_enumerate(items))  # {0: 'apple', 1: 'banana', 2: 'cherry'}

    # 多重代入テスト
    print(test_multi_assign_basic())       # (1, 2, 3)
    print(test_multi_assign_swap())        # (20, 10)
    print(test_multi_assign_from_tuple())  # (100, 200, 300)
    print(test_multi_assign_from_list())   # (1, 2, 3)
    print(test_multi_assign_nested())      # (1, 2, 3, 4)

    # 複合テスト
    print(test_dict_comp_with_multi_assign())  # {'a': 2, 'b': 4, 'c': 6}
    print(fibonacci_multi(10))  # [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]


if __name__ == "__main__":
    main()
