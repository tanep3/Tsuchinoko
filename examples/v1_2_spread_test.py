# v1_2_spread_test.py - Spread call test

from typing import List

def sum_all(*values: int) -> int:
    # 可変長引数を受け取って合計を返す
    total: int = 0
    for v in values:
        total += v
    return total


def apply_sum(nums: List[int]) -> int:
    # リストを展開して sum_all に渡す
    return sum_all(*nums)


def test_spread_call() -> None:
    result: int = apply_sum([10, 20, 30])
    print(f"apply_sum([10,20,30]) = {result}")


def main() -> None:
    print("=== Spread Call Test ===")
    test_spread_call()
    print("=== Done ===")


if __name__ == "__main__":
    main()
