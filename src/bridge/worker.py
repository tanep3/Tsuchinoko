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

# モジュールキャッシュ
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


def handle_call(request: dict) -> dict:
    """call 操作を処理"""
    target = request.get("target")
    args = request.get("args", [])
    kwargs = request.get("kwargs", {})
    
    if not target:
        return {"ok": False, "error": "Missing 'target' field"}
    
    try:
        func = get_callable(target)
        result = func(*args, **kwargs)
        
        # JSON 化できるか確認
        try:
            json.dumps(result)
            return {"ok": True, "result": result}
        except (TypeError, ValueError):
            # JSON 化できない場合は文字列化
            # numpy 配列などは tolist() を試す
            if hasattr(result, "tolist"):
                return {"ok": True, "result": result.tolist()}
            elif hasattr(result, "to_dict"):
                return {"ok": True, "result": result.to_dict()}
            else:
                return {"ok": True, "result": str(result)}
    
    except Exception as e:
        return {
            "ok": False,
            "error": str(e),
            "error_type": type(e).__name__,
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
        elif op == "ping":
            response = {"id": req_id, "ok": True, "result": "pong"}
        else:
            response = {"id": req_id, "ok": False, "error": f"Unknown op: {op}"}
        
        print(json.dumps(response), flush=True)


if __name__ == "__main__":
    main()
