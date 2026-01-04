# v1_2_default_test.py - デフォルト引数テスト

from typing import List, Optional

def take_n(values: List[int], limit: Optional[int] = None) -> List[int]:
    if limit is None:
        return values[:len(values)]
    return values[:limit]


def greet(name: str, greeting: str = "Hello") -> str:
    return f"{greeting}, {name}!"


def main() -> None:
    print("=== Default Arg Test ===")
    
    # Test 1: limit=None (use full list)
    all_vals: List[int] = take_n([1, 2, 3, 4, 5])
    print(f"take_n all: {all_vals}")
    
    # Test 2: limit=3
    limited: List[int] = take_n([1, 2, 3, 4, 5], 3)
    print(f"take_n 3: {limited}")
    
    # Test 3: string default
    msg1: str = greet("Tane")
    msg2: str = greet("Tane", "Hi")
    print(msg1)
    print(msg2)
    
    print("=== Done ===")


if __name__ == "__main__":
    main()
