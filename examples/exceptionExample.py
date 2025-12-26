def safe_divide(a: int, b: int) -> int:
    try:
        result: int = a // b
        return result
    except ZeroDivisionError:
        return -1

def main() -> None:
    x: int = safe_divide(10, 2)
    print(x)
    y: int = safe_divide(10, 0)
    print(y)

if __name__ == "__main__":
    main()
