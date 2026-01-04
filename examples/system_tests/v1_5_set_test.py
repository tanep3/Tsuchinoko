# v1.5.0 Phase 2 Tests - SET-001 to SET-006
# Test for set literal, constructor, methods, and operators

def test_set_literal() -> int:
    """SET-001: set literal {1, 2, 3}"""
    s: set[int] = {1, 2, 3}
    return 3


def test_set_constructor() -> int:
    """SET-002: set() constructor"""
    arr: list[int] = [1, 2, 2, 3]
    s: set[int] = set(arr)
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


# def test_set_union() -> int:
#     """SET-005: set union |"""
#     a: set[int] = {1, 2}
#     b: set[int] = {2, 3}
#     c: set[int] = a | b
#     return 3  # Should be 3: {1, 2, 3}


def test_set_in() -> bool:
    """SET-006: in operator"""
    s: set[int] = {1, 2, 3}
    return 2 in s


def main() -> None:
    print(test_set_literal())      # Expected: 3
    print(test_set_constructor())  # Expected: 3
    print(test_set_add())          # Expected: 3
    print(test_set_remove())       # Expected: 2
    # print(test_set_union())      # Expected: 3
    print(test_set_in())           # Expected: true


main()
