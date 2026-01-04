# v1.5.0 Exception Tests - Full Version
# Tests: multiple except, as variable, finally

def get_zero() -> int:
    """Helper to get zero at runtime"""
    return 0


def test_basic_try_except() -> int:
    """Basic try-except with panic"""
    result: int = 0
    try:
        zero: int = get_zero()
        x: int = 1 // zero
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


def test_except_type() -> int:
    """EX-001: except ValueError:"""
    result: int = 0
    try:
        zero: int = get_zero()
        x: int = 1 // zero
    except ValueError:
        result = 2
    return result


def test_multiple_except_types() -> int:
    """EX-002: except (TypeError, ValueError):"""
    result: int = 0
    try:
        zero: int = get_zero()
        x: int = 1 // zero
    except (TypeError, ValueError):
        result = 3
    return result


def test_except_as() -> int:
    """EX-003: except ValueError as e:"""
    result: int = 0
    try:
        zero: int = get_zero()
        x: int = 1 // zero
    except ValueError as e:
        result = 4
    return result


def test_finally() -> int:
    """EX-004: finally block"""
    result: int = 0
    try:
        x: int = 10
        result = x
    except:
        result = -1
    finally:
        result = result + 100
    return result


def test_finally_with_exception() -> int:
    """EX-004: finally executes even on exception"""
    result: int = 0
    try:
        zero: int = get_zero()
        x: int = 1 // zero
    except:
        result = 50
    finally:
        result = result + 100
    return result


def main() -> None:
    print(test_basic_try_except())        # Expected: 1
    print(test_try_success())              # Expected: 10
    print(test_except_type())              # Expected: 2
    print(test_multiple_except_types())    # Expected: 3
    print(test_except_as())                # Expected: 4
    print(test_finally())                  # Expected: 110
    print(test_finally_with_exception())   # Expected: 150


main()
