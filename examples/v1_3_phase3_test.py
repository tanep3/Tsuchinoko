"""
Phase 3 テスト: enumerate / zip
"""

def test_enumerate(items: list[str]) -> list[tuple[int, str]]:
    """基本的なenumerate"""
    result: list[tuple[int, str]] = []
    for i, item in enumerate(items):
        result.append((i, item))
    return result


def test_zip(a: list[int], b: list[str]) -> list[tuple[int, str]]:
    """基本的なzip"""
    result: list[tuple[int, str]] = []
    for x, y in zip(a, b):
        result.append((x, y))
    return result


def main() -> None:
    items: list[str] = []
    items.append("a")
    items.append("b")
    items.append("c")
    print(test_enumerate(items))
    
    nums: list[int] = [1, 2, 3]
    strs: list[str] = []
    strs.append("one")
    strs.append("two")
    strs.append("three")
    print(test_zip(nums, strs))


if __name__ == "__main__":
    main()
