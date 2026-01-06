# test_if_hoisting.py
# Variable defined inside if block, used outside

def check_value(n: int) -> int:
    if n > 0:
        result: int = n * 10
    else:
        result: int = -1
    
    # 'result' should be accessible here (Python semantics)
    return result

def main() -> None:
    print(check_value(5))    # Expected: 50
    print(check_value(-3))   # Expected: -1
    print(check_value(0))    # Expected: -1

if __name__ == "__main__":
    main()
