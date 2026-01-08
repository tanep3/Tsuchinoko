# V1.5.2 System Test: raise from syntax
# Tests exception chaining with "raise ... from ..." syntax

def validate_input(value: int) -> int:
    """Validate input value with exception chaining"""
    try:
        if value < 0:
            raise ValueError("negative value not allowed")
    except ValueError as e:
        raise RuntimeError("validation failed") from e
    return value * 2

def main():
    # Test normal case
    result = validate_input(5)
    print(result)  # Expected: 10
    
    # Test error case (will panic with cause chain)
    # result = validate_input(-1)  # Uncomment to test error

if __name__ == "__main__":
    main()
