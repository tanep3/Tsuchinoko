# v1_2_all_features_test.py - V1.2.0 综合テスト

from typing import List, Tuple, Optional


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
# 4. デフォルト引数 limit=None
# ============================================

def take_n(values: List[int], limit: Optional[int] = None) -> List[int]:
    if limit is None:
        return values.copy()
    return values[:limit]


# ============================================
# メイン
# ============================================

def main() -> None:
    print("=== V1.2.0 All Features Test ===")
    
    # Test 1: *args
    result1: int = sum_all(1, 2, 3, 4, 5)
    print(f"sum_all(1,2,3,4,5) = {result1}")
    
    # Test 2: star unpack
    h, t = head_and_tail([10, 20, 30, 40])
    print(f"head={h}")
    
    # Test 3: spread call
    result3: int = apply_sum([10, 20, 30])
    print(f"apply_sum([10,20,30]) = {result3}")
    
    # Test 4: default None
    all_vals: List[int] = take_n([1, 2, 3, 4, 5])
    limited: List[int] = take_n([1, 2, 3, 4, 5], 3)
    print(f"take_n all: len={len(all_vals)}")
    print(f"take_n 3: len={len(limited)}")
    
    print("=== All tests passed! ===")


if __name__ == "__main__":
    main()
