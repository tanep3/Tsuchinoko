#!/usr/bin/env python3
"""
v1_2_features_test.py
Tsuchinoko V1.2.0 新機能テスト用サンプル

対象機能:
1. *args パラメータ（可変長引数）
2. head, *tail = values（スターアンパック）
3. func(*args) 呼び出し（引数展開）
4. デフォルト引数 limit=None の正しい変換
5. @dataclass 基本対応
"""

from typing import Optional, List, Tuple
from dataclasses import dataclass


# ===========================================
# 1. *args パラメータ（可変長引数）
# ===========================================

def sum_all(*values: int) -> int:
    """可変長引数を受け取って合計を返す"""
    total: int = 0
    for v in values:
        total += v
    return total


def test_varargs() -> None:
    result: int = sum_all(1, 2, 3, 4, 5)
    print(f"sum_all(1,2,3,4,5) = {result}")


# ===========================================
# 2. head, *tail = values（スターアンパック）
# ===========================================

def head_and_tail(values: List[int]) -> Tuple[int, List[int]]:
    """先頭と残りを分離"""
    if len(values) == 0:
        return (0, [])
    head, *tail = values
    return (head, tail)


def test_star_unpack() -> None:
    h, t = head_and_tail([10, 20, 30, 40])
    print(f"head={h}")
    print(t)


# ===========================================
# 3. func(*args) 呼び出し（引数展開）
# ===========================================

def apply_sum(nums: List[int]) -> int:
    """リストを展開して sum_all に渡す"""
    return sum_all(*nums)


def test_spread_call() -> None:
    result: int = apply_sum([10, 20, 30])
    print(f"apply_sum([10,20,30]) = {result}")


# ===========================================
# 4. デフォルト引数 limit=None
# ===========================================

def take_n(values: List[int], limit: Optional[int] = None) -> List[int]:
    """limit が指定されていれば先頭 N 個、なければ全部返す"""
    if limit is None:
        return values
    return values[:limit]


def test_default_none() -> None:
    all_vals: List[int] = take_n([1, 2, 3, 4, 5])
    print(f"take_n([1..5]) len = {len(all_vals)}")
    
    limited: List[int] = take_n([1, 2, 3, 4, 5], 3)
    print(f"take_n([1..5], 3) len = {len(limited)}")


# ===========================================
# 5. @dataclass 基本対応
# ===========================================

@dataclass
class Point:
    x: int
    y: int


@dataclass
class Rectangle:
    top_left: Point
    width: int
    height: int


def area(rect: Rectangle) -> int:
    return rect.width * rect.height


def test_dataclass() -> None:
    p: Point = Point(x=10, y=20)
    print(f"Point: x={p.x}, y={p.y}")
    
    r: Rectangle = Rectangle(top_left=p, width=100, height=50)
    print(f"Rectangle area = {area(r)}")


# ===========================================
# メイン
# ===========================================

def main() -> None:
    print("=== V1.2.0 Features Test ===")
    print()
    
    print("[1] *args パラメータ")
    test_varargs()
    print()
    
    print("[2] スターアンパック")
    test_star_unpack()
    print()
    
    print("[3] 引数展開呼び出し")
    test_spread_call()
    print()
    
    print("[4] デフォルト引数 None")
    test_default_none()
    print()
    
    print("[5] @dataclass")
    test_dataclass()
    print()
    
    print("=== All tests passed! ===")


if __name__ == "__main__":
    main()
