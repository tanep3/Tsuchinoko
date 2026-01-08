# EX-008: 型推論精度向上 - 基本テスト
# シンプルな構造で型推論を検証
# if __name__ == "__main__" パターンを使わず直接実行

def get_value() -> int:
    """Return an integer"""
    return 42

def double_value(x: int) -> int:
    """Double the input value"""
    return x * 2

def sum_list(items: list[int]) -> int:
    """Sum items in list - for loop element type test"""
    total: int = 0
    for item in items:
        total = total + item
    return total

# Test A: 関数呼び出し戻り値型
x = get_value()
print(f"get_value() = {x}")

# Test A2: 関数チェーン
y = double_value(get_value())
print(f"double_value(get_value()) = {y}")

# Test B: for ループ要素型
numbers: list[int] = [1, 2, 3, 4, 5]
result = sum_list(numbers)
print(f"sum_list([1,2,3,4,5]) = {result}")
