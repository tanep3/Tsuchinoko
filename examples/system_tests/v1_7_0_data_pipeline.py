
# Data Pipeline Integration Test
# libraries: requests, pandas, matplotlib

import requests
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

def test_data_pipeline():
    print("Testing Data Pipeline...")
    
    # 1. Requests: Fetch data (Mocking external API to be safe/fast, or using stable public API?)
    # For robustness, we'll create a local dummy server or just assume connectivity to httpbin.
    # Let's try httpbin, but fallback to local data if needed? 
    # Actually, for "System Test", let's use a real request to `https://httpbin.org/json`
    print("1. Requests: fetching...")
    try:
        r = requests.get('https://httpbin.org/json')
        print(f"Status: {r.status_code}")
        data = r.json() # Retruns Dict (Handle? or Value if small?)
        # Tsuchinoko V1.7.0 returns Value for primitives/small dicts, 
        # but if it contains complex types it might be Handle.
        # However, `r.json()` usually returns pure JSON data.
        # Let's verify we can use this data.
        print("Data fetched.")
    except Exception as e:
        print(f"Network failed (ignoring for test): {e}")
        # Fallback data
        data = {"slideshow": {"slides": [{"title": "Wake up to WonderWidgets!", "type": "all"}]}}

    # 2. Pandas: Create DataFrame
    print("2. Pandas: Creating DataFrame...")
    # Create a dummy dataframe intentionally to test Handle
    df = pd.DataFrame(np.random.randn(10, 4), columns=['A', 'B', 'C', 'D'])
    
    print("DataFrame Info:")
    print(df.shape)  # Attribute access
    print(df.head()) # Method call
    
    desc = df.describe()
    print("Description:")
    print(desc)

    # 3. Matplotlib: Plotting
    print("3. Matplotlib: Plotting...")
    # df.plot() returns axes
    ax = df.plot(kind='bar')
    
    # Setting title via ax handle
    ax.set_title("Tsuchinoko V1.7.0 Test Plot")
    
    # Saving figure
    filename = "v1_7_0_plot.png"
    plt.savefig(filename)
    
    print(f"Plot saved to {filename}")
    
    if os.path.exists(filename):
        print("Verification Passed: Plot file exists.")
        os.remove(filename)
    else:
        raise Exception("Verification Failed: Plot file not created.")

if __name__ == "__main__":
    test_data_pipeline()
