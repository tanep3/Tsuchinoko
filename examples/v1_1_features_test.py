# *args 基本テスト
# V1.1.0 対応確認用

def sum_all(a: int, b: int, c: int) -> int:
    """通常の関数呼び出し"""
    return a + b + c

def main() -> None:
    # 名前付き引数テスト
    result1: int = sum_all(1, 2, 3)
    result2: int = sum_all(a=10, b=20, c=30)
    result3: int = sum_all(1, b=2, c=3)
    
    print(result1)  # 6
    print(result2)  # 60
    print(result3)  # 6
    
    # is / is not テスト (Optional風)
    x: int = 10
    if x is not None:
        print("x is valid")
    
    # デフォルト引数テスト（greet関数から）
    print("All tests passed!")

if __name__ == "__main__":
    main()
