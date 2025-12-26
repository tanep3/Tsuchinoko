# Tsuchinoko 変換テスト：網羅スモーク + 地雷踏み
# 目的：Rust変換で壊れやすい要素（len/index/負数/空/多値返し/短絡/内包/スライス/列挙/関数/例外相当の分岐）を一括で通す

from typing import Optional

def safe_div(a: int, b: int) -> Optional[int]:
    # ゼロ除算相当（RustではOptionが欲しいやつ）
    if b == 0:
        return None
    return a // b

def stats(nums: list[int]) -> tuple[int, int, int, list[int]]:
    # 空配列処理（len==0）、負数、インデックス、sum/min/max、内包表記、早期return
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

    # 内包表記（変換しにくいなら for で展開してもOK）
    doubled: list[int] = [x * 2 for x in nums]
    return total, mn, mx, doubled

def bubble_sort_inplace(a: list[int]) -> None:
    # in-place更新（&mutが必要）、早期終了フラグ、len/index、空でも壊れない
    n: int = len(a)
    i: int = 0
    j: int = 0

    for i in range(n):
        swapped: bool = False
        for j in range(n - i - 1):
            if a[j] > a[j + 1]:
                a[j], a[j + 1] = a[j + 1], a[j]   # タプルswap（Rust変換の見どころ）
                swapped = True
        if not swapped:
            break

def slice_ops(nums: list[int]) -> tuple[list[int], list[int], list[int]]:
    # スライス（範囲）、負のインデックス（Rustで地雷）、境界
    # 変換器が未対応なら「未対応として弾く」用のテストにもなる
    head3: list[int] = nums[:3]
    tail3: list[int] = nums[-3:]     # ←負のindex由来（要注意）
    mid: list[int] = nums[1:len(nums)-1]
    return head3, tail3, mid

def find_first_even(nums: list[int]) -> int:
    # break/continue/早期return
    for x in nums:
        if x < 0:
            continue
        if x % 2 == 0:
            return x
    return -1

def program_start() -> None:
    # 空 / 1要素 / 重複 / 負数 / 大きめ
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

        first_even = find_first_even(t)
        print("first_even:", first_even)

        # cloneしてソート（所有権・コピーの再現）
        a: list[int] = list(t)
        bubble_sort_inplace(a)
        print("sorted:", a)

        # Option相当
        d = safe_div(total, len(t))
        print("avg_floor_or_none:", d)

        # slice（未対応ならここで弾ける）
        head3, tail3, mid = slice_ops(t)
        print("slices:", head3, tail3, mid)

if __name__ == "__main__":
    program_start()
