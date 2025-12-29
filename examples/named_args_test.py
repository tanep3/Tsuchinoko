# Test file for named arguments and default arguments

def greet(name: str, greeting: str = "Hello") -> str:
    return f"{greeting}, {name}!"

def add(a: int, b: int = 10, c: int = 20) -> int:
    result: int = a + b + c
    return result

def main() -> None:
    # Test default arguments
    msg1: str = greet("World")
    msg2: str = greet("Tane", "Hi")
    
    # Test named arguments
    msg3: str = greet(name="User", greeting="Welcome")
    msg4: str = greet(greeting="Hey", name="Friend")
    
    # Test with numeric defaults
    sum1: int = add(1)
    sum2: int = add(1, 2)
    sum3: int = add(1, 2, 3)
    sum4: int = add(a=5, b=15, c=25)
    
    print(msg1)
    print(msg2)
    print(msg3)
    print(msg4)
    print(sum1)
    print(sum2)
    print(sum3)
    print(sum4)

if __name__ == "__main__":
    main()
