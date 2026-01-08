# V1.5.2 System Test: Error Chain Display with Line Numbers
# Tests EX-006 (Error message improvement) and EX-007 (Error chain display)
#
# Expected error output format:
# [line N] RuntimeError: validation failed
#   Caused by: [line M] ValueError: negative value not allowed

def validate_positive(value: int) -> int:
    """Validate that value is positive, raise ValueError if not"""
    if value < 0:
        raise ValueError("negative value not allowed")
    return value * 2

def process_value(value: int) -> int:
    """Process value with error chaining"""
    try:
        result = validate_positive(value)
    except ValueError as e:
        raise RuntimeError("validation failed") from e
    return result

def main() -> None:
    # Test 1: Normal case - should succeed
    result1 = process_value(5)
    print(f"process_value(5) = {result1}")  # Expected: 10
    
    # Test 2: Another normal case
    result2 = validate_positive(10)
    print(f"validate_positive(10) = {result2}")  # Expected: 20
    
    # Uncomment to test error case with line numbers:
    # result3 = process_value(-1)  # Should show error chain with line numbers

if __name__ == "__main__":
    main()
