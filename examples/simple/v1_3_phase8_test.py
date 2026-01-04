"""
V1.3.0追加機能テスト: @演算子と辞書内包表記
"""


def test_dict_comp_basic(items: list[int]) -> dict[int, int]:
    """基本的な辞書内包表記"""
    return {i: i * 2 for i in items}


def test_dict_comp_with_condition(items: list[int]) -> dict[int, int]:
    """条件付き辞書内包表記"""
    return {i: i * i for i in items if i > 0}


def main() -> None:
    nums: list[int] = [1, 2, 3, 4, 5]
    
    # 辞書内包表記テスト
    result1: dict[int, int] = test_dict_comp_basic(nums)
    print(result1)  # {1: 2, 2: 4, 3: 6, 4: 8, 5: 10}
    
    mixed: list[int] = [-2, -1, 0, 1, 2, 3]
    result2: dict[int, int] = test_dict_comp_with_condition(mixed)
    print(result2)  # {1: 1, 2: 4, 3: 9}


if __name__ == "__main__":
    main()
