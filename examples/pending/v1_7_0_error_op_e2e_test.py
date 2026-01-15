# Tsuchinokoは辞書の整数キーアクセス（data[0]）しか対応しておらず、文字列キーアクセス（data["key"]）は未対応です。
# しかし、これを診断する機能がありません。
# 根本対応するか、診断対応するかは未定です。

# Test for Worker Error op field - E2E Test
# This test verifies error.op field through actual Rust-Python Worker communication

import numpy as np

def cause_index_error():
    """Cause IndexError to verify error.op includes cmd/target/key"""
    arr = np.array([1, 2, 3, 4, 5])
    # This will cause IndexError when accessed via Worker's get_item
    result = arr[999]  # Index out of range
    return result

def cause_attribute_error():
    """Cause AttributeError to verify error.op includes cmd/target/name"""
    arr = np.array([1, 2, 3])
    # This will cause AttributeError when accessed via Worker's get_attribute
    result = arr.nonexistent_attribute
    return result

def cause_key_error():
    """Cause KeyError to verify error.op includes cmd/target/key"""
    data = {"a": 1, "b": 2}
    # This will cause KeyError when accessed via Worker's get_item
    result = data["nonexistent_key"]
    return result

# Main test - this will be executed by Rust and should trigger errors with op field
try:
    cause_index_error()
except IndexError as e:
    print(f"IndexError caught: {e}")

try:
    cause_attribute_error()
except AttributeError as e:
    print(f"AttributeError caught: {e}")

try:
    cause_key_error()
except KeyError as e:
    print(f"KeyError caught: {e}")

print("E2E error op test completed")
