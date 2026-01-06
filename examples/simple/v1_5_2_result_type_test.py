# test_result_type.py
# System test for Result type unification (v1.5.2)
# Functions with raise should return Result<T, TsuchinokoError>

def may_fail(value: int) -> int:
    if value < 0:
        raise ValueError("Negative values not allowed")
    return value * 2

def main() -> None:
    # Test normal case
    result = may_fail(5)
    print(f"may_fail(5) = {result}")  # Expected: 10
    
    # Test error case - this will now be a proper Result::Err
    # result = may_fail(-1)  # Would return Err

if __name__ == "__main__":
    main()
