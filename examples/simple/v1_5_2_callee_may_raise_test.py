# V1.5.2 System Test: Simple callee_may_raise test
# Tests that Result propagation works correctly without try/except
# This is a simpler test to verify basic may_raise functionality

def may_fail(value: int) -> int:
    """A function that may raise"""
    if value < 0:
        raise ValueError("negative value")
    return value * 2

def caller(x: int) -> int:
    """Calls may_fail - should propagate Result"""
    result = may_fail(x)
    return result + 10

def main() -> None:
    # Test 1: Direct call to may_raise function
    r1 = may_fail(5)
    print(f"may_fail(5) = {r1}")  # Expected: 10
    
    # Test 2: Nested call chain
    r2 = caller(3)
    print(f"caller(3) = {r2}")  # Expected: 16

if __name__ == "__main__":
    main()
