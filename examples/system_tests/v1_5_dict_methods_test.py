# v1.5.0 Phase 2-C Tests - Dict Methods (Full)

def test_dict_keys() -> int:
    """DCT-001: .keys() - returns iterable of keys"""
    d: dict[int, str] = {1: "a", 2: "b", 3: "c"}
    keys: list[int] = list(d.keys())
    return len(keys)  # Expected: 3


def test_dict_values() -> int:
    """DCT-002: .values() - returns iterable of values"""
    d: dict[int, str] = {1: "a", 2: "b", 3: "c"}
    values: list[str] = list(d.values())
    return len(values)  # Expected: 3


def test_dict_get() -> str:
    """DCT-003: .get(k) - returns value or None"""
    d: dict[int, str] = {1: "a", 2: "b"}
    val: str = d.get(1)
    return val  # Expected: "a"


def test_dict_get_default() -> str:
    """DCT-004: .get(k, default) - returns value or default"""
    d: dict[int, str] = {1: "a", 2: "b"}
    val: str = d.get(99, "default")
    return val  # Expected: "default"


def test_dict_pop() -> str:
    """DCT-005: .pop(k) - removes and returns value"""
    d: dict[int, str] = {1: "a", 2: "b", 3: "c"}
    val: str = d.pop(2)
    return val  # Expected: "b"


def test_dict_update() -> int:
    """DCT-006: .update(other) - merges other dict"""
    d: dict[int, str] = {1: "a", 2: "b"}
    d.update({3: "c", 4: "d"})
    return len(d)  # Expected: 4


def main() -> None:
    print(test_dict_keys())        # Expected: 3
    print(test_dict_values())      # Expected: 3
    print(test_dict_get())         # Expected: a
    print(test_dict_get_default()) # Expected: default
    print(test_dict_pop())         # Expected: b
    print(test_dict_update())      # Expected: 4


main()
