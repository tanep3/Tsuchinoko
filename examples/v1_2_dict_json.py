
# Test dictionary generation and function propagation
import pandas as pd

def process_data(data):
    # This function uses resident feature (pandas), so it should maximize py_bridge
    df = pd.DataFrame(data)
    print(df)
    return df

def main():
    print("=== Dict Test ===")
    # Heterogeneous dict
    data = {"name": ["Alice", "Bob"], "score": [80, 90]}
    
    # Pass to function that uses resident feature
    process_data(data)
    
    print("=== Done ===")

if __name__ == "__main__":
    main()
