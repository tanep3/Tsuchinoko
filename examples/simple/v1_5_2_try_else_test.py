# test_hoisting.py
# Variable hoisting test - variable defined in try block, used in else block

def safe_divide(a: int, b: int) -> int:
    try:
        result: int = a // b
    except:
        return -1
    else:
        return result

def main() -> None:
    print(safe_divide(10, 2))   # Expected: 5
    print(safe_divide(10, 0))   # Expected: -1

if __name__ == "__main__":
    main()
