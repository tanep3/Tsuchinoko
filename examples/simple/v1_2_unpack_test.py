# v1_2_unpack_test.py - Star unpack test

from typing import List, Tuple

def head_and_tail(values: List[int]) -> Tuple[int, List[int]]:
    # 先頭と残りを分離
    if len(values) == 0:
        return (0, [])
    head, *tail = values
    return (head, tail)


def test_star_unpack() -> None:
    h, t = head_and_tail([10, 20, 30, 40])
    print(f"head={h}, tail={t}")


def main() -> None:
    print("=== Star Unpack Test ===")
    test_star_unpack()
    print("=== Done ===")


if __name__ == "__main__":
    main()
