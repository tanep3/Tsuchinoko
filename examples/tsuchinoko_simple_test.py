# Simplified test for Tsuchinoko - workaround version
# (swapped declared outside for loop, no len() as function arg)

from typing import Optional

def safe_div(a: int, b: int) -> Optional[int]:
    if b == 0:
        return None
    return a // b

def stats(nums: list[int]) -> tuple[int, int, int, list[int]]:
    if len(nums) == 0:
        return 0, 0, 0, []

    total: int = 0
    mn: int = nums[0]
    mx: int = nums[0]

    i: int = 0
    for i in range(len(nums)):
        x: int = nums[i]
        total += x
        if x < mn:
            mn = x
        if x > mx:
            mx = x

    doubled: list[int] = [x * 2 for x in nums]
    return total, mn, mx, doubled

def bubble_sort_simple(a: list[int]) -> list[int]:
    # Return sorted copy instead of in-place modification
    result: list[int] = list(a)
    n: int = len(result)
    i: int = 0
    j: int = 0
    swapped: bool = False  # Declared outside for loop

    for i in range(n):
        swapped = False
        for j in range(n - i - 1):
            if result[j] > result[j + 1]:
                result[j], result[j + 1] = result[j + 1], result[j]
                swapped = True
        if not swapped:
            break
    return result

def slice_ops(nums: list[int]) -> tuple[list[int], list[int], list[int]]:
    head3: list[int] = nums[:3]
    tail3: list[int] = nums[-3:]
    mid: list[int] = nums[1:len(nums)-1]
    return head3, tail3, mid

def find_first_even(nums: list[int]) -> int:
    for x in nums:
        if x < 0:
            continue
        if x % 2 == 0:
            return x
    return -1

def program_start() -> None:
    tests: list[list[int]] = [
        [],
        [1],
        [2, 2, 2],
        [3, -1, 4, 0, 5],
        [64, 34, 25, 12, 22, 11, 90],
    ]

    for t in tests:
        print("----")
        print("input:", t)
        total, mn, mx, doubled = stats(t)
        print("stats:", total, mn, mx, doubled)
        first_even: int = find_first_even(t)
        print("first_even:", first_even)
        sorted_list: list[int] = bubble_sort_simple(t)
        print("sorted:", sorted_list)
        n: int = len(t)
        d: Optional[int] = safe_div(total, n)
        print("avg_floor_or_none:", d)
        h, tl, m = slice_ops(t)
        print("slices:", h, tl, m)

if __name__ == "__main__":
    program_start()
