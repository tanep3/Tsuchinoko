import pandas as pd

def main():
    print("Founding Tsuchinoko Pandas Test")

    # 1. DataFrameの作成 (BridgeMethodCall)
    # 辞書からの作成
    data = {
        "id": [1, 2, 3, 4, 5],
        "value": [10.5, 20.0, 30.5, 40.0, 50.5],
        "category": ["A", "B", "A", "B", "C"]
    }
    df = pd.DataFrame(data)
    
    # 文字列変換して表示 (中身の確認)
    print("--- DataFrame Content ---")
    print(df.to_string())

    # 2. 属性アクセス (BridgeAttributeAccess)
    print("\n--- Columns ---")
    cols = df.columns
    # Indexオブジェクトをリストに変換して表示
    print(cols.to_list())

    print("\n--- Shape ---")
    shape = df.shape
    # タプルが返るはず (5, 3)
    # タプルはContainerとして展開されるか、Handleとして返るか？
    # v1.7.0の仕様では、Primitiveな要素のタプルはValueとして返るはず (Phase 0検証事項)
    # しかしValue::Tuple対応は実装次第。とりあえずprintしてみる
    print(shape)

    # 3. メソッド呼び出し (BridgeMethodCall)
    print("\n--- Head(2) ---")
    head_df = df.head(2)
    print(head_df.to_string())

    print("\n--- Describe ---")
    desc = df.describe()
    print(desc.to_string())

    # 4. アイテムアクセス (BridgeItemAccess)
    print("\n--- Column 'value' ---")
    # df["value"]
    val_series = df["value"]
    print(val_series.to_list())

    # 5. スライス (BridgeSlice)
    print("\n--- Slice [1:4:2] ---")
    # 1行目から4行目まで2行おき -> row 1, 3 (id 2, 4)
    slice_df = df[1:4:2]
    print(slice_df.to_string())

    # 6. 集計と検証
    print("\n--- Aggregation ---")
    total_val = val_series.sum()
    print("Total Value:", total_val)
    
    # 期待値: 10.5 + 20.0 + 30.5 + 40.0 + 50.5 = 151.5
    # f64の比較
    if total_val != 151.5:
        print("ERROR: Sum mismatch!")
        print("Expected: 151.5")
        print("Got:", total_val)
        raise ValueError("Verification Failed")
    
    print("\nVerification Passed!")

if __name__ == "__main__":
    main()
