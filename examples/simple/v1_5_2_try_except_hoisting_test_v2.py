# test_try_except_hoisting_v2.py
# Variable defined in try block, return inside try/except (both branches)

def process_data_v2(value: int) -> str:
    try:
        result: str = f"Success: {value * 2}"
        x: int = 100 // value  # May cause division by zero
        return result  # Return inside try
    except:
        # 'result' should be accessible here even though defined in try
        return f"Error: result was {result}"

def main() -> None:
    print(process_data_v2(5))   # Expected: Success: 10
    print(process_data_v2(0))   # Expected: Error: result was Success: 0

if __name__ == "__main__":
    main()
