# 条件付きリスト内包表記テスト

def main() -> None:
    # 偶数のみをフィルタ
    numbers: list[int] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    evens: list[int] = [x for x in numbers if x % 2 == 0]
    print(evens)
    
    # 条件付きでマップ
    squared_evens: list[int] = [x * x for x in numbers if x % 2 == 0]
    print(squared_evens)
    
    # rangeでの条件付き
    divisible_by_3: list[int] = [i for i in range(1, 20) if i % 3 == 0]
    print(divisible_by_3)

if __name__ == "__main__":
    main()
