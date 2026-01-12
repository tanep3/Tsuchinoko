# Formatting Comprehensive Test

def main() -> None:
    # 1. String
    s: str = "Hello"
    print(f"String: {s}") # Expected: String: Hello
    
    # 2. Int
    i: int = 123
    print(f"Int: {i}") # Expected: Int: 123
    
    # 3. Set (Native Rust type)
    st: set[int] = {1, 2, 3}
    print(f"Set: {st}") # Expected: Set: {1, 2, 3} (or similar rust debug output)
    
    # 4. Any (TnkValue in standalone)
    # Using a list to force Any? No, list[int] is known.
    # How to get Any in standalone? Maybe a function that returns Any?
    # Actually, standalone TnkValue::from(String) is what FizzBuzz4 used.
    
    import math
    a: any = math.pi
    print(f"Any (float): {a}") # Expected: Any (float): 3.14159...

    b: any = "Fizz"
    print(f"Any (string): {b}") # Expected: Any (string): Fizz (No quotes!)

if __name__ == "__main__":
    main()
