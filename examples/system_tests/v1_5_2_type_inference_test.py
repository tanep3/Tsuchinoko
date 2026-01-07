# EX-008: 型推論精度向上 システムテスト
# TDD: このテストは実装前に作成され、最初は失敗する
#
# テスト対象:
# - 関数呼び出し戻り値型
# - for ループ要素型
# - タプル展開
# - メソッド呼び出し戻り値型
# - may_raise 関数の戻り値型

def get_number() -> int:
    """Return an integer"""
    return 42

def get_pair() -> tuple[int, str]:
    """Return a tuple"""
    return (10, "hello")

def process_list(items: list[int]) -> int:
    """Sum items in list - tests for loop element type"""
    total = 0
    for item in items:  # item should be inferred as int
        total = total + item  # item + int should work
    return total

class Calculator:
    def __init__(self) -> None:
        self.value = 0
    
    def add(self, x: int) -> int:
        """Add x to value and return result"""
        self.value = self.value + x
        return self.value

def may_raise_func() -> int:
    """A function that may raise"""
    if False:
        raise ValueError("error")
    return 100

def use_may_raise() -> int:
    """Call may_raise_func and use result"""
    result = may_raise_func()  # result should be int, not Result<int, E>
    return result + 1  # This should work: int + int

def main() -> None:
    # Test A: Function call return type
    x = get_number()  # x should be int
    print(f"get_number() = {x}")
    
    # Test C: Tuple unpacking
    a, b = get_pair()  # a should be int, b should be str
    print(f"get_pair() = ({a}, {b})")
    
    # Test B: For loop element type
    numbers = [1, 2, 3, 4, 5]
    total = process_list(numbers)
    print(f"process_list([1,2,3,4,5]) = {total}")  # Expected: 15
    
    # Test D: Method call return type
    calc = Calculator()
    result = calc.add(5)  # result should be int
    print(f"calc.add(5) = {result}")
    
    # Test H: may_raise function return type
    result_final = use_may_raise()  # should work without type errors
    print(f"use_may_raise() = {result_final}")  # Expected: 101

if __name__ == "__main__":
    main()
