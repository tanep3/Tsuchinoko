# V1.6.0 セット内包表記テスト

def get_unique_squares(items: list[int]) -> set[int]:
    return {x * x for x in items}

def get_even_numbers(items: list[int]) -> set[int]:
    return {x for x in items if x % 2 == 0}

def program_start() -> None:
    nums: list[int] = [1, 2, 3, 4, 5, 2, 3, 4]  # 重複あり
    
    # Test 1: 基本セット内包表記
    squares: set[int] = get_unique_squares(nums)
    print(f"squares: {squares}")  # {1, 4, 9, 16, 25}
    
    # Test 2: 条件付きセット内包表記
    evens: set[int] = get_even_numbers(nums)
    print(f"evens: {evens}")  # {2, 4}
    
    # Test 3: 重複排除確認
    unique: set[int] = {x for x in nums}
    print(f"unique (from list with duplicates): {unique}")  # {1, 2, 3, 4, 5}

if __name__ == "__main__":
    program_start()
