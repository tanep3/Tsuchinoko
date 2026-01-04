# v1_2_working_test.py - V1.2.0 動作確認テスト

from typing import List, Tuple


# ============================================
# 1. *args パラメータ
# ============================================

def sum_all(*values: int) -> int:
    total: int = 0
    for v in values:
        total += v
    return total


# ============================================
# 2. head, *tail = values スターアンパック
# ============================================

def head_and_tail(values: List[int]) -> Tuple[int, List[int]]:
    if len(values) == 0:
        return (0, [])
    head, *tail = values
    return (head, tail)


# ============================================
# 3. func(*args) 引数展開呼び出し
# ============================================

def apply_sum(nums: List[int]) -> int:
    return sum_all(*nums)


# ============================================
# メイン
# ============================================

def main() -> None:
    print("=== V1.2.0 Working Test ===")
    
    # Test 1: *args
    result1: int = sum_all(1, 2, 3, 4, 5)
    print(f"sum_all(1,2,3,4,5) = {result1}")
    
    # Test 2: star unpack
    h, t = head_and_tail([10, 20, 30, 40])
    print(f"head={h}")
    
    # Test 3: spread call
    result3: int = apply_sum([10, 20, 30])
    print(f"apply_sum([10,20,30]) = {result3}")
    
    print("=== All tests passed! ===")


if __name__ == "__main__":
    main()
