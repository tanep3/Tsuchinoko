import pandas as pd
import numpy as np

def test_pandas_method():
    data = {"A": [1, 2, 3], "B": [4, 5, 6]}
    df = pd.DataFrame(data)
    
    # 1. メソッド呼び出し (to_numpy)
    arr = df.to_numpy()
    
    # 2. インデックスアクセス
    col_a = df["A"]
    
    # 3. 再度のメソッド呼び出し (mean) - np.mean(df["A"])
    m = np.mean(col_a)
    
    print(f"Mean: {m}")
    return m

if __name__ == "__main__":
    m = test_pandas_method()
    assert m == 2.0
    print("Test Passed!")
