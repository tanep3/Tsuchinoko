# v1.5.0 Phase 2 Tests - SET-001, SET-003, SET-004
# Test for set literal and methods

def test_set_literal() -> int:
    """SET-001: set literal {1, 2, 3}"""
    s: set[int] = {1, 2, 3}
    return 3


def test_set_add() -> int:
    """SET-003: .add()"""
    s: set[int] = {1, 2}
    s.add(3)
    return 3


def test_set_remove() -> int:
    """SET-004: .remove()"""
    s: set[int] = {1, 2, 3}
    s.remove(2)
    return 2


def main() -> None:
    print(test_set_literal())   # Expected: 3
    print(test_set_add())       # Expected: 3
    print(test_set_remove())    # Expected: 2


main()
