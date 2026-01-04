# v1_2_early_return.py - 早期リターン後の Type Narrowing テスト

from typing import List, Optional

def take_n(values: List[int], limit: Optional[int] = None) -> List[int]:
    if limit is None:
        return values[:len(values)]
    # ここで limit は int 型にナローイングされるべき
    return values[:limit]


def main() -> None:
    print("=== Early Return Narrowing Test ===")
    
    all_vals: List[int] = take_n([1, 2, 3, 4, 5])
    print(f"take_n all: len={len(all_vals)}")
    
    limited: List[int] = take_n([1, 2, 3, 4, 5], 3)
    print(f"take_n 3: len={len(limited)}")
    
    print("=== Done ===")


if __name__ == "__main__":
    main()
