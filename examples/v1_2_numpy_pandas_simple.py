#!/usr/bin/env python3
"""
v1_2_numpy_pandas_simple.py
Tsuchinoko V1.2.0: numpy + pandas 最小テスト（PyO3ブリッジ想定）

方針:
- numpy.typing / 型エイリアスを使わない（NDInt禁止）
- int()/float() を使わない（未変換問題を回避）
- df[df["score"]>=...] のような高度インデックスを使わない
- .tolist(), .shape, .dtype などのPythonメソッド連鎖を避ける
- ただし型ヒントは付ける（np.ndarray / pd.DataFrame）
"""

import numpy as np
import pandas as pd


def test_numpy() -> None:
    print("=== numpy test ===")

    a: np.ndarray = np.array([1, 2, 3, 4, 5], dtype=np.int64)
    s: object = np.sum(a)              # int()/float()に触れないためobjectで受ける
    print("sum(a) =", s)

    m: np.ndarray = np.array([[1, 2], [3, 4]], dtype=np.int64)
    n: np.ndarray = np.array([[5, 6], [7, 8]], dtype=np.int64)
    p: np.ndarray = np.dot(m, n)       # @ は使わない
    print("dot(m,n) =")
    print(p)


def test_pandas() -> None:
    print("\n=== pandas test ===")

    df: pd.DataFrame = pd.DataFrame(
        {"name": ["Alice", "Bob", "Charlie"], "score": [80, 92, 85]}
    )

    # 文字列化して表示（フィルタ等はしない）
    txt: str = df.to_string(index=False)
    print(txt)


def test_interop() -> None:
    print("\n=== interop test ===")

    mat: np.ndarray = np.array([[10, 20], [30, 40]], dtype=np.int64)

    # numpy -> pandas
    df2: pd.DataFrame = pd.DataFrame(mat, columns=["c0", "c1"])
    print(df2.to_string(index=False))

    # pandas -> numpy
    back: np.ndarray = df2.to_numpy()
    print("back =")
    print(back)


def main() -> None:
    test_numpy()
    test_pandas()
    test_interop()
    print("\n=== done ===")


if __name__ == "__main__":
    main()
