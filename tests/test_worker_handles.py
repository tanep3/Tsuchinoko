import sys
import os
import json

# worker.py があるディレクトリをパスに追加
sys.path.append(os.path.join(os.path.dirname(__file__), "..", "src", "bridge"))

import worker

def test_handle_management():
    # 1. オブジェクトの保存
    data = {"key": "value"}
    handle = worker.store_object(data)
    assert "__t_handle" in handle
    
    # 2. オブジェクトの取得
    obj = worker.get_object(handle)
    assert obj == data
    
    # 3. シリアライズ (JSON化可能な場合)
    simple = [1, 2, 3]
    res_simple = worker.serialize_result(simple)
    assert res_simple == simple
    
    # 4. シリアライズ (JSON化不可な場合 -> ハンドル返却)
    class NonSerializable:
        pass
    
    ns = NonSerializable()
    res_ns = worker.serialize_result(ns)
    assert isinstance(res_ns, dict)
    assert "__t_handle" in res_ns
    
    # 取得して同一性を確認
    assert worker.get_object(res_ns) is ns
    print("Worker Handle Test Passed!")

if __name__ == "__main__":
    test_handle_management()
