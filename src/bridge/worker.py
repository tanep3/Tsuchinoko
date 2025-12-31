"""
Tsuchinoko Python Worker
========================

常駐プロセス方式で Rust バイナリから呼び出される Python ワーカー。
stdin/stdout で NDJSON 通信を行い、任意の Python ライブラリを呼び出す。

このコードは Rust 側に文字列として埋め込まれ、
`python -u -c "<WORKER_CODE>"` で起動される。
"""

import sys
import json
import importlib
import traceback

# オブジェクトハンドル管理
_objects = {}
_next_id = 0
_modules_cache = {}


def get_callable(target: str):
    """
    target 文字列から callable を解決する。
    例: "math.sqrt" -> math.sqrt
        "numpy.linalg.norm" -> numpy.linalg.norm
    """
    parts = target.split(".")
    if len(parts) < 2:
        raise ValueError(f"Invalid target: {target}")
    
    # 最短モジュール名から試行（例: "numpy" を先に試す）
    for i in range(1, len(parts)):
        module_name = ".".join(parts[:i])
        attr_path = parts[i:]
        
        # キャッシュまたは新規 import
        if module_name not in _modules_cache:
            try:
                _modules_cache[module_name] = importlib.import_module(module_name)
            except ImportError:
                continue
        
        # 属性チェーンを辿る
        obj = _modules_cache[module_name]
        try:
            for attr in attr_path:
                obj = getattr(obj, attr)
            return obj
        except AttributeError:
            continue
    
    raise ValueError(f"Cannot resolve target: {target}")

def store_object(obj):
    global _next_id
    id_str = str(_next_id)
    _objects[id_str] = obj
    _next_id += 1
    return {"__t_handle": id_str}

def get_object(handle_data):
    if isinstance(handle_data, dict) and "__t_handle" in handle_data:
        id_str = handle_data["__t_handle"]
        if id_str in _objects:
            return _objects[id_str]
    raise ValueError(f"Invalid handle: {handle_data}")

def serialize_result(result, force_value=False):
    """結果を JSON 化可能な形式に変換。変換できない場合はハンドルを返す。
    
    force_value=True の場合、ハンドルではなく値を返す（表示目的）。
    """
    if result is None or isinstance(result, (int, float, bool, str)):
        return result
    
    # NumPy 配列の場合はリストに変換（常に）
    try:
        import numpy as np
        if isinstance(result, np.ndarray):
            return result.tolist()
        if isinstance(result, (np.integer, np.floating)):
            return result.item()
    except ImportError:
        pass
    
    # Pandas DataFrame/Series の場合
    try:
        import pandas as pd
        if isinstance(result, pd.DataFrame):
            # DataFrame はメソッドチェーンに使うのでハンドルとして保持
            # ただし to_string() 等の結果は文字列として返される
            return store_object(result)
        if isinstance(result, pd.Series):
            return result.tolist()
    except ImportError:
        pass
    
    # JSON 化を試行
    try:
        # 基本的なコレクションは再帰的にチェックすべきだが、ここでは一気に dumps して確認
        json.dumps(result)
        return result
    except (TypeError, ValueError):
        # JSON 化できない場合はハンドルとして保存
        return store_object(result)

def handle_call(request: dict) -> dict:
    """call 操作を処理"""
    target = request.get("target")
    args = request.get("args", [])
    kwargs = request.get("kwargs", {})
    
    if not target:
        return {"ok": False, "error": "Missing 'target' field"}
    
    try:
        # 引数にハンドルが含まれる場合は解決
        processed_args = [get_object(a) if isinstance(a, dict) and "__t_handle" in a else a for a in args]
        
        func = get_callable(target)
        result = func(*processed_args, **kwargs)
        
        return {"ok": True, "result": serialize_result(result)}
    
    except Exception as e:
        return {
            "ok": False,
            "error": f"{type(e).__name__}: {str(e)}",
            "traceback": traceback.format_exc()
        }

def handle_method(request: dict) -> dict:
    """method 操作を処理 (ハンドルに対してメソッド呼び出し)"""
    handle = request.get("handle")
    method_name = request.get("method")
    args = request.get("args", [])
    kwargs = request.get("kwargs", {})
    
    try:
        obj = get_object(handle)
        method = getattr(obj, method_name)
        
        # 引数にハンドルが含まれる場合は解決
        processed_args = [get_object(a) if isinstance(a, dict) and "__t_handle" in a else a for a in args]
        
        result = method(*processed_args, **kwargs)
        return {"ok": True, "result": serialize_result(result)}
    except Exception as e:
        return {
            "ok": False,
            "error": f"{type(e).__name__}: {str(e)}",
            "traceback": traceback.format_exc()
        }


def main():
    """メインループ: stdin から NDJSON を読み、処理して stdout へ返す"""
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        
        try:
            request = json.loads(line)
        except json.JSONDecodeError as e:
            response = {"id": None, "ok": False, "error": f"JSON parse error: {e}"}
            print(json.dumps(response), flush=True)
            continue
        
        req_id = request.get("id")
        op = request.get("op", "call")
        
        if op == "shutdown":
            response = {"id": req_id, "ok": True, "result": "shutdown"}
            print(json.dumps(response), flush=True)
            break
        elif op == "call":
            response = handle_call(request)
            response["id"] = req_id
        elif op == "method":
            response = handle_method(request)
            response["id"] = req_id
        elif op == "ping":
            response = {"id": req_id, "ok": True, "result": "pong"}
        else:
            response = {"id": req_id, "ok": False, "error": f"Unknown op: {op}"}
        
        print(json.dumps(response), flush=True)


if __name__ == "__main__":
    main()
