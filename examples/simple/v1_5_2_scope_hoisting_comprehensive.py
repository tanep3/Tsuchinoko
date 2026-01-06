# test_scope_hoisting_comprehensive.py
# Comprehensive tests for Python scope semantics in transpiled Rust
# Python does NOT have block scope for if/for/while/try
# Variables defined inside these blocks MUST be accessible outside

# =============================================================================
# Test 1: if block - variable defined in if, used after
# =============================================================================
def test_if_simple() -> int:
    if True:
        x: int = 42
    return x  # Must work: x is accessible

# =============================================================================
# Test 2: if/else - variable defined in both branches, used after
# =============================================================================
def test_if_else(flag: bool) -> int:
    if flag:
        result: int = 100
    else:
        result: int = -100
    return result  # Must work: result is accessible

# =============================================================================
# Test 3: for loop - loop variable accessible after loop
# =============================================================================
def test_for_loop_var() -> int:
    for i in range(5):
        pass
    return i  # Must work: i is accessible (last value = 4)

# =============================================================================
# Test 4: for loop - variable defined inside loop, used after
# =============================================================================
def test_for_inner_var() -> int:
    for n in range(3):
        last: int = n * 10
    return last  # Must work: last is accessible (20)

# =============================================================================
# Test 5: while loop - variable defined inside, used after
# =============================================================================
def test_while_inner() -> int:
    count: int = 0
    while count < 3:
        value: int = count * 2
        count = count + 1
    return value  # Must work: value is accessible (4)

# =============================================================================
# Test 6: Nested blocks - deeply nested variable accessible at top level
# =============================================================================
def test_nested() -> int:
    if True:
        for i in range(2):
            if i > 0:
                deep: int = i * 100
    return deep  # Must work: deep is accessible (100)

# =============================================================================
# Test 7: try/except - variable defined in try, used in except
# =============================================================================
def test_try_except(val: int) -> str:
    try:
        result: str = f"Value: {val}"
        x: int = 10 // val  # May raise
    except:
        return f"Error, partial: {result}"
    return result

# =============================================================================
# Test 8: try/else - variable defined in try, used in else
# =============================================================================
def test_try_else(a: int, b: int) -> int:
    try:
        result: int = a // b
    except:
        return -1
    else:
        return result  # Must work: result accessible in else

# =============================================================================
# Main: Run all tests
# =============================================================================
def main() -> None:
    print(f"test_if_simple: {test_if_simple()}")           # Expected: 42
    print(f"test_if_else(True): {test_if_else(True)}")     # Expected: 100
    print(f"test_if_else(False): {test_if_else(False)}")   # Expected: -100
    print(f"test_for_loop_var: {test_for_loop_var()}")     # Expected: 4
    print(f"test_for_inner_var: {test_for_inner_var()}")   # Expected: 20
    print(f"test_while_inner: {test_while_inner()}")       # Expected: 4
    print(f"test_nested: {test_nested()}")                 # Expected: 100
    print(f"test_try_except(5): {test_try_except(5)}")     # Expected: Value: 5
    print(f"test_try_except(0): {test_try_except(0)}")     # Expected: Error, partial: Value: 0
    print(f"test_try_else(10, 2): {test_try_else(10, 2)}") # Expected: 5
    print(f"test_try_else(10, 0): {test_try_else(10, 0)}") # Expected: -1

if __name__ == "__main__":
    main()
