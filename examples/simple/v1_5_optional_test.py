# v1.5.0 Phase 6 Tests - None / Optional Deep Support
# Tests for: or pattern, and short-circuit, ternary combined

def test_or_with_none() -> str:
    """OPT-001: x or default - returns default when x is None"""
    x: str | None = None
    result: str = x or "default"
    return result  # Expected: "default"


def test_or_with_value() -> str:
    """OPT-001: x or default - returns x when x has value"""
    x: str | None = "actual"
    result: str = x or "default"
    return result  # Expected: "actual"


def test_or_with_empty_string() -> str:
    """OPT-001: empty string is falsy in Python"""
    x: str = ""
    result: str = x or "default"
    return result  # Expected: "default" (empty string is falsy)


def test_and_short_circuit_none() -> int:
    """OPT-002: x and x.method() - short-circuits when x is None"""
    x: list[int] | None = None
    result: int = len(x) if x else 0
    return result  # Expected: 0


def test_and_short_circuit_value() -> int:
    """OPT-002: x and x.method() - evaluates when x has value"""
    x: list[int] | None = [1, 2, 3]
    result: int = len(x) if x else 0
    return result  # Expected: 3


def test_ternary_with_none() -> str:
    """OPT-003: ternary with None check"""
    x: str | None = None
    result: str = x if x is not None else "fallback"
    return result  # Expected: "fallback"


def test_ternary_with_value() -> str:
    """OPT-003: ternary with value"""
    x: str | None = "hello"
    result: str = x if x is not None else "fallback"
    return result  # Expected: "hello"


def main() -> None:
    print(test_or_with_none())         # Expected: default
    print(test_or_with_value())        # Expected: actual
    print(test_or_with_empty_string()) # Expected: default
    print(test_and_short_circuit_none())  # Expected: 0
    print(test_and_short_circuit_value()) # Expected: 3
    print(test_ternary_with_none())    # Expected: fallback
    print(test_ternary_with_value())   # Expected: hello


main()
