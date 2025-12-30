#!/usr/bin/env python3
"""
numpy_pandas_test.py
Tsuchinoko V1.2.0 外部ライブラリテスト用サンプル

対象:
- numpy: 配列操作
- pandas: DataFrame操作

NOTE: このファイルはPyO3ブリッジ経由での実行を想定。
純Rustへのトランスパイルではなく、PyO3でPythonライブラリを呼び出す。
"""

from typing import List, Tuple
import numpy as np
import pandas as pd


# ===========================================
# 1. NumPy 基本操作
# ===========================================

def create_array(values: List[int]) -> np.ndarray:
    """リストからNumPy配列を作成"""
    return np.array(values)


def array_stats(arr: np.ndarray) -> Tuple[float, float, float]:
    """配列の統計値を返す: 平均, 標準偏差, 合計"""
    return (float(np.mean(arr)), float(np.std(arr)), float(np.sum(arr)))


def matrix_multiply(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """行列積"""
    return np.dot(a, b)


def test_numpy() -> None:
    print("[NumPy] 配列作成")
    arr: np.ndarray = create_array([1, 2, 3, 4, 5])
    print(f"  array: {arr}")
    
    print("[NumPy] 統計")
    mean, std, total = array_stats(arr)
    print(f"  mean={mean}, std={std:.2f}, sum={total}")
    
    print("[NumPy] 行列積")
    a: np.ndarray = np.array([[1, 2], [3, 4]])
    b: np.ndarray = np.array([[5, 6], [7, 8]])
    c: np.ndarray = matrix_multiply(a, b)
    print(f"  A @ B = {c.tolist()}")


# ===========================================
# 2. Pandas 基本操作
# ===========================================

def create_dataframe(data: dict) -> pd.DataFrame:
    """辞書からDataFrameを作成"""
    return pd.DataFrame(data)


def filter_dataframe(df: pd.DataFrame, column: str, threshold: int) -> pd.DataFrame:
    """指定列が閾値以上の行をフィルタ"""
    return df[df[column] >= threshold]


def aggregate_dataframe(df: pd.DataFrame, group_col: str, agg_col: str) -> pd.DataFrame:
    """グループ別集計"""
    return df.groupby(group_col)[agg_col].sum().reset_index()


def test_pandas() -> None:
    print("[Pandas] DataFrame作成")
    data = {
        "name": ["Alice", "Bob", "Charlie", "David"],
        "age": [25, 30, 35, 28],
        "score": [85, 90, 78, 92],
        "team": ["A", "B", "A", "B"]
    }
    df: pd.DataFrame = create_dataframe(data)
    print(df.to_string(index=False))
    
    print("\n[Pandas] フィルタ: score >= 85")
    filtered: pd.DataFrame = filter_dataframe(df, "score", 85)
    print(filtered.to_string(index=False))
    
    print("\n[Pandas] グループ集計: team別 score合計")
    agg: pd.DataFrame = aggregate_dataframe(df, "team", "score")
    print(agg.to_string(index=False))


# ===========================================
# 3. NumPy + Pandas 連携
# ===========================================

def numpy_to_pandas(arr: np.ndarray, columns: List[str]) -> pd.DataFrame:
    """NumPy配列をDataFrameに変換"""
    return pd.DataFrame(arr, columns=columns)


def pandas_to_numpy(df: pd.DataFrame) -> np.ndarray:
    """DataFrameをNumPy配列に変換"""
    return df.to_numpy()


def test_interop() -> None:
    print("[連携] NumPy -> Pandas")
    arr: np.ndarray = np.array([[1, 2, 3], [4, 5, 6], [7, 8, 9]])
    df: pd.DataFrame = numpy_to_pandas(arr, ["A", "B", "C"])
    print(df.to_string(index=False))
    
    print("\n[連携] Pandas -> NumPy")
    back: np.ndarray = pandas_to_numpy(df)
    print(f"  shape: {back.shape}, dtype: {back.dtype}")


# ===========================================
# メイン
# ===========================================

def main() -> None:
    print("=== NumPy/Pandas Test ===")
    print()
    
    test_numpy()
    print()
    
    test_pandas()
    print()
    
    test_interop()
    print()
    
    print("=== All tests passed! ===")


if __name__ == "__main__":
    main()
