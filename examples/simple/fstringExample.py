# f-string テスト

def main() -> None:
    name: str = "Tsuchinoko"
    version: int = 5
    
    # Simple f-string
    message: str = f"Hello, {name}!"
    print(message)
    
    # Multiple interpolations
    info: str = f"Version {version}: {name}"
    print(info)
    
    # F-string with expression
    x: int = 10
    y: int = 20
    result: str = f"Sum: {x + y}"
    print(result)

if __name__ == "__main__":
    main()
