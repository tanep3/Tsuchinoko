# V1.6.0 連鎖比較テスト

def in_range(x: int) -> bool:
    return 0 < x < 10

def check_bounds(low: int, val: int, high: int) -> bool:
    return low <= val <= high

def triple_check(a: int, b: int, c: int, d: int) -> bool:
    return a < b < c < d

def program_start() -> None:
    # Test 1: Simple chained comparison
    print("Test 1: 0 < 5 < 10 =", in_range(5))   # True
    print("Test 2: 0 < 15 < 10 =", in_range(15)) # False
    print("Test 3: 0 < 0 < 10 =", in_range(0))   # False (0 < 0 is False)
    
    # Test 2: Range check with <=
    print("Test 4: 1 <= 5 <= 10 =", check_bounds(1, 5, 10))   # True
    print("Test 5: 1 <= 0 <= 10 =", check_bounds(1, 0, 10))   # False
    
    # Test 3: Triple chained comparison
    print("Test 6: 1 < 2 < 3 < 4 =", triple_check(1, 2, 3, 4))   # True
    print("Test 7: 1 < 2 < 2 < 4 =", triple_check(1, 2, 2, 4))   # False

if __name__ == "__main__":
    program_start()
