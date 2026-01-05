# v1.5.0 Phase 2-A Tests - Set Type
# Tests for set literal, constructor, methods, and operators
# Note: Returns actual set lengths/contains results, not magic numbers

def test_set_literal() -> int:
    """SET-001: set literal {1, 2, 3}"""
    s: set[int] = {1, 2, 3}
    return len(s)  # Expected: 3


def test_set_constructor() -> int:
    """SET-002: set() constructor - deduplicates"""
    arr: list[int] = [1, 2, 2, 3, 3, 3]
    s: set[int] = set(arr)
    return len(s)  # Expected: 3 (duplicates removed)


def test_set_add() -> int:
    """SET-003: .add() - adds element if not present"""
    s: set[int] = {1, 2}
    s.add(3)
    s.add(2)  # Already exists, should not increase size
    return len(s)  # Expected: 3


def test_set_remove() -> int:
    """SET-004: .remove() - removes element"""
    s: set[int] = {1, 2, 3}
    s.remove(2)
    return len(s)  # Expected: 2


def test_set_union() -> int:
    """SET-005: set union | - combines sets"""
    a: set[int] = {1, 2}
    b: set[int] = {2, 3}
    c: set[int] = a | b
    return len(c)  # Expected: 3 ({1, 2, 3})


def test_set_intersection() -> int:
    """SET-005: set intersection & - common elements"""
    a: set[int] = {1, 2, 3}
    b: set[int] = {2, 3, 4}
    c: set[int] = a & b
    return len(c)  # Expected: 2 ({2, 3})


def test_set_difference() -> int:
    """SET-005: set difference - - elements in a but not b"""
    a: set[int] = {1, 2, 3}
    b: set[int] = {2, 3}
    c: set[int] = a - b
    return len(c)  # Expected: 1 ({1})


def test_set_in() -> bool:
    """SET-006: in operator - membership check"""
    s: set[int] = {1, 2, 3}
    return 2 in s  # Expected: true


def test_set_not_in() -> bool:
    """SET-006: not in operator - membership check"""
    s: set[int] = {1, 2, 3}
    return 5 not in s  # Expected: true


def main() -> None:
    print(test_set_literal())       # Expected: 3
    print(test_set_constructor())   # Expected: 3
    print(test_set_add())           # Expected: 3
    print(test_set_remove())        # Expected: 2
    print(test_set_union())         # Expected: 3
    print(test_set_intersection())  # Expected: 2
    print(test_set_difference())    # Expected: 1
    print(test_set_in())            # Expected: true
    print(test_set_not_in())        # Expected: true


main()
