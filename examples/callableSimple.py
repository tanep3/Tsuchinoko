# 簡単なCallableテスト - 1引数版

def double(x: int) -> int:
    return x * 2

# fn(i64) -> i64 として変換されるはず
def apply_simple(func: Callable[[int], int], val: int) -> int:
    return func(val)

def main() -> None:
    result: int = apply_simple(double, 5)
    print(result)

if __name__ == "__main__":
    main()
