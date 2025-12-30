# v1_2_narrowing_simple.py - シンプルな Type Narrowing テスト

from typing import Optional

def process_opt(value: Optional[int] = None) -> int:
    if value is None:
        return 0
    else:
        # ここで value は int 型にナローイング
        return value + 10


def main() -> None:
    print("=== Type Narrowing Test ===")
    
    r1: int = process_opt()
    print(f"process_opt() = {r1}")
    
    r2: int = process_opt(5)
    print(f"process_opt(5) = {r2}")
    
    print("=== Done ===")


if __name__ == "__main__":
    main()
