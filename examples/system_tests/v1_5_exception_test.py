# v1.5.0 Exception Tests - Basic Version
# Test current try-except capabilities

def get_zero() -> int:
    """Helper to get zero at runtime"""
    return 0


def test_basic_try_except() -> int:
    """Basic try-except with panic"""
    result: int = 0
    try:
        # Use function call to avoid compile-time detection
        zero: int = get_zero()
        x: int = 1 // zero  # Will cause panic at runtime
    except:
        result = 1
    return result


def test_try_success() -> int:
    """Test try block without exception"""
    result: int = 0
    try:
        x: int = 10
        result = x
    except:
        result = -1
    return result


def test_finally_simple() -> int:
    """EX-004: finally block simulation"""
    result: int = 0
    try:
        x: int = 10
        result = x
    except:
        result = -1
    # finally equivalent: code after try-except
    result = result + 1
    return result


def main() -> None:
    print(test_basic_try_except())  # Expected: 1 (caught exception)
    print(test_try_success())        # Expected: 10 (no exception)
    print(test_finally_simple())     # Expected: 11


main()
