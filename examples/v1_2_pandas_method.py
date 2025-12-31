import pandas as pd
import numpy as np

def main():
    # DataFrame 作成 (Any 型)
    data = {
        'name': ['Alice', 'Bob', 'Charlie'],
        'age': [25, 30, 35]
    }
    df = pd.DataFrame(data)
    
    # メソッド呼び出し (Any 型へのメソッド呼び出し -> PyO3MethodCall)
    # これが Rust 側で df.to_numpy() -> py_bridge.call_json_method(...) に変換される
    arr = df.to_numpy()
    
    # Numpy 操作
    mean_age = np.mean(df['age'])
    
    print("DataFrame:")
    print(df)
    print("\nNumpy Array (from to_numpy):")
    print(arr)
    print("\nMean Age:", mean_age)
