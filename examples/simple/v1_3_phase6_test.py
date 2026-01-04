"""
Phase 6 テスト: all/any/map/filter（シンプル版）
"""

def test_all_simple(flags: list[bool]) -> bool:
    """allはリストをそのまま受け取る"""
    return all(flags)


def test_any_simple(flags: list[bool]) -> bool:
    """anyはリストをそのまま受け取る"""
    return any(flags)


def test_map(nums: list[int]) -> list[int]:
    """全て2倍"""
    return list(map(lambda x: x * 2, nums))


def test_filter(nums: list[int]) -> list[int]:
    """正の値のみ"""
    return list(filter(lambda x: x > 0, nums))


def main() -> None:
    flags_all_true: list[bool] = [True, True, True]
    flags_mixed: list[bool] = [True, False, True]
    
    print(test_all_simple(flags_all_true))  # True
    print(test_all_simple(flags_mixed))     # False
    print(test_any_simple(flags_mixed))     # True
    
    nums: list[int] = [1, 2, 3, 4, 5]
    print(test_map(nums))    # [2, 4, 6, 8, 10]
    
    nums2: list[int] = [1, -2, 3, -4, 5]
    print(test_filter(nums2))  # [1, 3, 5]


if __name__ == "__main__":
    main()
