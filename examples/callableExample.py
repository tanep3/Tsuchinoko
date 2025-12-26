from typing import Callable

# 条件関数型
ConditionFunction = Callable[[int, int], bool]

def is_divisible(x: int, y: int) -> bool:
    return x % y == 0

def apply_condition(func: Callable[[int, int], bool], a: int, b: int) -> bool:
    return func(a, b)

def main() -> None:
    result: bool = apply_condition(is_divisible, 10, 2)
    print(result)
    
    result2: bool = apply_condition(is_divisible, 10, 3)
    print(result2)

if __name__ == "__main__":
    main()
