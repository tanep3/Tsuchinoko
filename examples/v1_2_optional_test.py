# v1_2_optional_test.py - Optional 型と Type Narrowing テスト

from typing import List, Optional

def take_n(values: List[int], limit: Optional[int] = None) -> List[int]:
    if limit is None:
        return values[:len(values)]
    # ここで limit は int 型にナローイングされるべき
    return values[:limit]


def process(data: Optional[str] = None) -> str:
    if data is not None:
        # ここで data は str 型にナローイングされるべき
        return data
    return "default"


def main() -> None:
    print("=== Optional Type Narrowing Test ===")
    
    # Test 1: None passed (use full)
    all_vals: List[int] = take_n([1, 2, 3, 4, 5])
    print(f"take_n all: len={len(all_vals)}")
    
    # Test 2: limit passed
    limited: List[int] = take_n([1, 2, 3, 4, 5], 3)
    print(f"take_n 3: len={len(limited)}")
    
    # Test 3: process with None
    r1: str = process()
    print(f"process() = {r1}")
    
    # Test 4: process with value
    r2: str = process("hello")
    print(f"process('hello') = {r2}")
    
    print("=== Done ===")


if __name__ == "__main__":
    main()
