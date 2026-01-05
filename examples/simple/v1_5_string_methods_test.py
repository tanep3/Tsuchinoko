# v1.5.0 Phase 3 Tests - String Methods
# Tests for string methods: replace, starts/endswith, find, isXXX, padding

def test_str_replace() -> str:
    """STR-001: .replace(old, new)"""
    s: str = "hello world"
    result: str = s.replace("world", "rust")
    return result  # Expected: "hello rust"


def test_str_startswith() -> bool:
    """STR-002: .startswith()"""
    s: str = "hello world"
    return s.startswith("hello")  # Expected: true


def test_str_endswith() -> bool:
    """STR-002: .endswith()"""
    s: str = "hello world"
    return s.endswith("world")  # Expected: true


def test_str_find() -> int:
    """STR-003: .find()"""
    s: str = "hello world"
    return s.find("wor")  # Expected: 6


def test_str_rfind() -> int:
    """STR-003: .rfind()"""
    s: str = "hello hello"
    return s.rfind("hello")  # Expected: 6


def test_str_isdigit() -> bool:
    """STR-004: .isdigit()"""
    s: str = "12345"
    return s.isdigit()  # Expected: true


def test_str_isalpha() -> bool:
    """STR-004: .isalpha()"""
    s: str = "hello"
    return s.isalpha()  # Expected: true


def test_str_isalnum() -> bool:
    """STR-004: .isalnum()"""
    s: str = "hello123"
    return s.isalnum()  # Expected: true


def test_str_isupper() -> bool:
    """STR-005: .isupper()"""
    s: str = "HELLO"
    return s.isupper()  # Expected: true


def test_str_islower() -> bool:
    """STR-005: .islower()"""
    s: str = "hello"
    return s.islower()  # Expected: true


def test_str_zfill() -> str:
    """STR-006: .zfill()"""
    s: str = "42"
    return s.zfill(5)  # Expected: "00042"


def test_str_ljust() -> str:
    """STR-006: .ljust()"""
    s: str = "hi"
    return s.ljust(5)  # Expected: "hi   "


def test_str_rjust() -> str:
    """STR-006: .rjust()"""
    s: str = "hi"
    return s.rjust(5)  # Expected: "   hi"


def test_str_center() -> str:
    """STR-006: .center()"""
    s: str = "hi"
    return s.center(6)  # Expected: "  hi  "


def test_str_count() -> int:
    """STR-007: .count(sub)"""
    s: str = "banana"
    return s.count("a")  # Expected: 3


def main() -> None:
    print(test_str_replace())     # Expected: hello rust
    print(test_str_startswith())  # Expected: true
    print(test_str_endswith())    # Expected: true
    print(test_str_find())        # Expected: 6
    print(test_str_rfind())       # Expected: 6
    print(test_str_isdigit())     # Expected: true
    print(test_str_isalpha())     # Expected: true
    print(test_str_isalnum())     # Expected: true
    print(test_str_isupper())     # Expected: true
    print(test_str_islower())     # Expected: true
    print(test_str_zfill())       # Expected: 00042
    print(test_str_ljust())       # Expected: "hi   "
    print(test_str_rjust())       # Expected: "   hi"
    print(test_str_center())      # Expected: "  hi  "
    print(test_str_count())       # Expected: 3


main()
