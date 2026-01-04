# v1_2_import_test.py - import テスト

from typing import List
import math


def calc_sqrt(x: float) -> float:
    return math.sqrt(x)


def main() -> None:
    print("=== Import Test ===")
    result: float = calc_sqrt(16.0)
    print(f"sqrt(16) = {result}")
    print("=== Done ===")


if __name__ == "__main__":
    main()
