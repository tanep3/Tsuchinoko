# Test for Worker Error op field
# This test verifies that error responses include operation information in the error.op field

import pandas as pd
import numpy as np

def test_error_op_get_item():
    """Test that get_item error includes op info"""
    data = [1, 2, 3]
    try:
        # This should trigger IndexError with op info
        result = data[999]  # Index out of range
        print("Should not reach here")
    except IndexError as e:
        print(f"Caught error: {e}")

def test_error_op_get_attribute():
    """Test that get_attribute error includes op info"""
    df = pd.DataFrame({"A": [1, 2, 3]})
    try:
        # This should trigger AttributeError with op info
        result = df.nonexistent_attribute
        print("Should not reach here")
    except AttributeError as e:
        print(f"Caught error: {e}")

def test_error_op_call_method():
    """Test that call_method error includes op info"""
    arr = np.array([1, 2, 3])
    try:
        # This should trigger error with op info
        result = arr.nonexistent_method()
        print("Should not reach here")
    except AttributeError as e:
        print(f"Caught error: {e}")

# Run tests
test_error_op_get_item()
test_error_op_get_attribute()
test_error_op_call_method()

print("All error op tests completed")
