# Minimal lambda test for Tsuchinoko

def main() -> None:
    # Basic lambda - inline
    double = lambda x: x * 2
    result1: int = double(5)
    print("double(5):", result1)
    
    # Lambda with no params
    get_42 = lambda: 42
    result2: int = get_42()
    print("get_42():", result2)
    
    # Multiple params
    add = lambda x, y: x + y
    result3: int = add(3, 4)
    print("add(3, 4):", result3)

if __name__ == "__main__":
    main()
