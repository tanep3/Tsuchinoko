# v1.5.0 Phase 2-B Tests - List Methods
# Tests for list pop, insert, remove, extend, clear

def test_list_pop() -> int:
    """LST-001: .pop() - removes and returns last element"""
    arr: list[int] = [1, 2, 3]
    last: int = arr.pop()
    return last  # Expected: 3


def test_list_pop_index() -> int:
    """LST-002: .pop(i) - removes and returns element at index i"""
    arr: list[int] = [1, 2, 3]
    middle: int = arr.pop(1)
    return middle  # Expected: 2


def test_list_insert() -> int:
    """LST-003: .insert(i, x) - inserts x at index i"""
    arr: list[int] = [1, 3]
    arr.insert(1, 2)
    return arr[1]  # Expected: 2


def test_list_remove() -> int:
    """LST-004: .remove(x) - removes first occurrence of x"""
    arr: list[int] = [1, 2, 3, 2]
    arr.remove(2)
    return len(arr)  # Expected: 3


def test_list_extend() -> int:
    """LST-005: .extend(iter) - adds all elements from iterable"""
    arr: list[int] = [1, 2]
    arr.extend([3, 4])
    return len(arr)  # Expected: 4


def test_list_clear() -> int:
    """LST-006: .clear() - removes all elements"""
    arr: list[int] = [1, 2, 3]
    arr.clear()
    return len(arr)  # Expected: 0


def main() -> None:
    print(test_list_pop())        # Expected: 3
    print(test_list_pop_index())  # Expected: 2
    print(test_list_insert())     # Expected: 2
    print(test_list_remove())     # Expected: 3
    print(test_list_extend())     # Expected: 4
    print(test_list_clear())      # Expected: 0


main()
