"""
Phase 7 テスト: assert
"""

def test_assert_basic(x: int) -> int:
    """基本的なassert"""
    assert x > 0
    return x * 2


def test_assert_with_message(x: int) -> int:
    """メッセージ付きassert"""
    assert x >= 0, "x must be non-negative"
    return x + 1


def main() -> None:
    print(test_assert_basic(5))        # 10
    print(test_assert_with_message(0)) # 1


if __name__ == "__main__":
    main()
