# v1.5.0 Phase 4 Tests - Built-in Functions
# Tests for: round, chr, ord, bin, hex, oct
# Note: input() is interactive and cannot be tested automatically

def test_round_int() -> int:
    """BLT-002: round(x) - rounds to nearest int"""
    x: float = 3.7
    return round(x)  # Expected: 4


def test_round_ndigits() -> float:
    """BLT-002: round(x, n) - rounds to n decimal places"""
    x: float = 3.14159
    return round(x, 2)  # Expected: 3.14


def test_chr() -> str:
    """BLT-003: chr(n) - returns character for code point"""
    return chr(65)  # Expected: "A"


def test_ord() -> int:
    """BLT-003: ord(c) - returns code point for character"""
    return ord("A")  # Expected: 65


def test_bin() -> str:
    """BLT-004: bin(x) - returns binary string"""
    return bin(10)  # Expected: "0b1010"


def test_hex() -> str:
    """BLT-004: hex(x) - returns hex string"""
    return hex(255)  # Expected: "0xff"


def test_oct() -> str:
    """BLT-004: oct(x) - returns octal string"""
    return oct(64)  # Expected: "0o100"


def main() -> None:
    print(test_round_int())       # Expected: 4
    print(test_round_ndigits())   # Expected: 3.14
    print(test_chr())             # Expected: A
    print(test_ord())             # Expected: 65
    print(test_bin())             # Expected: 0b1010
    print(test_hex())             # Expected: 0xff
    print(test_oct())             # Expected: 0o100


main()
