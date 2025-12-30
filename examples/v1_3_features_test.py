#!/usr/bin/env python3
"""
v1_3_features_test.py
Tsuchinoko V1.3.0 新機能テスト用サンプル

対象機能:
1. collections.Counter 型ヒント
2. collections.defaultdict 型ヒント
3. collections.deque 型ヒント
"""

from typing import List, Tuple, Dict
from collections import Counter, defaultdict, deque


# ===========================================
# 1. Counter 型ヒント
# ===========================================

def count_words(words: List[str]) -> List[Tuple[str, int]]:
    """単語の出現回数をカウント"""
    counter: Counter[str] = Counter(words)
    return counter.most_common(3)


def test_counter() -> None:
    words: List[str] = ["apple", "banana", "apple", "cherry", "banana", "apple"]
    top3 = count_words(words)
    print(f"Top 3 words: {top3}")


# ===========================================
# 2. defaultdict 型ヒント
# ===========================================

def group_by_length(words: List[str]) -> Dict[int, List[str]]:
    """単語を長さでグループ化"""
    groups: defaultdict[int, List[str]] = defaultdict(list)
    for word in words:
        groups[len(word)].append(word)
    return dict(groups)


def test_defaultdict() -> None:
    words: List[str] = ["a", "bb", "ccc", "dd", "eee", "f"]
    grouped = group_by_length(words)
    print(f"Grouped by length: {grouped}")


# ===========================================
# 3. deque 型ヒント
# ===========================================

def sliding_window_max(values: List[int], window_size: int) -> List[int]:
    """スライディングウインドウの最大値"""
    result: List[int] = []
    d: deque[int] = deque()
    
    for i in range(len(values)):
        # 古い要素を削除
        if d and d[0] <= i - window_size:
            d.popleft()
        # 現在の値より小さい要素を削除
        while d and values[d[-1]] < values[i]:
            d.pop()
        d.append(i)
        
        if i >= window_size - 1:
            result.append(values[d[0]])
    
    return result


def test_deque() -> None:
    values: List[int] = [1, 3, -1, -3, 5, 3, 6, 7]
    result = sliding_window_max(values, 3)
    print(f"Sliding window max: {result}")


# ===========================================
# メイン
# ===========================================

def main() -> None:
    print("=== V1.3.0 Features Test ===")
    print()
    
    print("[1] Counter")
    test_counter()
    print()
    
    print("[2] defaultdict")
    test_defaultdict()
    print()
    
    print("[3] deque")
    test_deque()
    print()
    
    print("=== All tests passed! ===")


if __name__ == "__main__":
    main()
