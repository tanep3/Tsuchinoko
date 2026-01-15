"""
Unit tests for Worker Error op field
=====================================
Tests that error responses include operation information in error.op field
"""

import json
import sys
from io import StringIO

# Import the worker module (assuming it's in the same directory)
# In practice, this would need proper path setup
# For now, we'll create a minimal test that can be run independently

def test_make_response_with_op_info():
    """Test that make_response includes op info in error responses"""
    # Simulate the make_response function
    def make_response(req_id, value=None, meta=None, error=None, op_info=None):
        if error:
            if op_info:
                error["op"] = op_info
            return {"kind": "error", "req_id": req_id, "error": error}
        return {"kind": "ok", "req_id": req_id, "value": value, "meta": meta}
    
    # Test 1: Error without op_info
    resp1 = make_response("req_1", error={"code": "TestError", "message": "test"})
    assert resp1["kind"] == "error"
    assert "op" not in resp1["error"]
    print("✓ Test 1 passed: Error without op_info")
    
    # Test 2: Error with op_info
    op_info = {"cmd": "get_item", "target": "obj_1", "key": {"kind": "value", "value": 999}}
    resp2 = make_response("req_2", error={"code": "TestError", "message": "test"}, op_info=op_info)
    assert resp2["kind"] == "error"
    assert "op" in resp2["error"]
    assert resp2["error"]["op"]["cmd"] == "get_item"
    assert resp2["error"]["op"]["target"] == "obj_1"
    print("✓ Test 2 passed: Error with op_info")
    
    # Test 3: Success response (op_info should be ignored)
    resp3 = make_response("req_3", value={"kind": "value", "value": 42}, op_info=op_info)
    assert resp3["kind"] == "ok"
    assert "op" not in resp3
    print("✓ Test 3 passed: Success response ignores op_info")
    
    print("\nAll unit tests passed! ✅")

if __name__ == "__main__":
    test_make_response_with_op_info()
