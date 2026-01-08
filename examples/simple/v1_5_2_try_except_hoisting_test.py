# test_try_except_hoisting.py
# Variable defined in try block, used in except block

def process_data(value: int) -> str:
    try:
        result: str = f"Success: {value * 2}"
        x: int = 100 // value  # May cause division by zero
    except:
        # 'result' should be accessible here even though defined in try
        return f"Error occurred, partial result was: {result}"
    return result

def main() -> None:
    print(process_data(5))   # Expected: Success: 10
    print(process_data(0))   # Expected: Error occurred, partial result was: Success: 0

if __name__ == "__main__":
    main()
