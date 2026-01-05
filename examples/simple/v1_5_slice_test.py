# v1.5.0 Phase 5 Tests - Slice Complete Support
# Tests for: step slicing, reverse slicing, range+step slicing (list AND string)

def test_slice_step() -> list[int]:
    """SLC-001: arr[::2] - every 2nd element"""
    arr: list[int] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    return arr[::2]  # Expected: [0, 2, 4, 6, 8]


def test_slice_reverse() -> list[int]:
    """SLC-002: arr[::-1] - reverse the list"""
    arr: list[int] = [1, 2, 3, 4, 5]
    return arr[::-1]  # Expected: [5, 4, 3, 2, 1]


def test_slice_range_step() -> list[int]:
    """SLC-003: arr[1:8:2] - range with step"""
    arr: list[int] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    return arr[1:8:2]  # Expected: [1, 3, 5, 7]


def test_str_slice_step() -> str:
    """SLC-001 for string: s[::2]"""
    s: str = "abcdefghij"
    return s[::2]  # Expected: "acegi"


def test_str_reverse() -> str:
    """SLC-002 for string: s[::-1]"""
    s: str = "hello"
    return s[::-1]  # Expected: "olleh"


def main() -> None:
    print(test_slice_step())       # Expected: [0, 2, 4, 6, 8]
    print(test_slice_reverse())    # Expected: [5, 4, 3, 2, 1]
    print(test_slice_range_step()) # Expected: [1, 3, 5, 7]
    print(test_str_slice_step())   # Expected: acegi
    print(test_str_reverse())      # Expected: olleh


main()
