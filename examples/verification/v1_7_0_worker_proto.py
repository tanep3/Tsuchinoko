import sys
import json
import uuid
import traceback
import math

# --- Global Object Store ---
_OBJECT_STORE = {}

def get_session_store(session_id):
    if session_id not in _OBJECT_STORE:
        _OBJECT_STORE[session_id] = {}
    return _OBJECT_STORE[session_id]

# --- Protocol Helpers ---
def make_response(req_id, value=None, meta=None, error=None):
    if error:
        return {"kind": "error", "req_id": req_id, "error": error}
    return {"kind": "ok", "req_id": req_id, "value": value, "meta": meta}

def encode_value(v, session_id):
    """Encode a Python value to TnkValue."""
    if v is None:
        return {"kind": "value", "value": None}
    if isinstance(v, bool):
        return {"kind": "value", "value": v}
    if isinstance(v, (int, float)):
        return {"kind": "value", "value": v}
    if isinstance(v, str):
        return {"kind": "value", "value": v}
    if isinstance(v, list):
        return {"kind": "list", "items": [encode_value(x, session_id) for x in v]}
    if isinstance(v, tuple):
        return {"kind": "tuple", "items": [encode_value(x, session_id) for x in v]}
    if isinstance(v, dict):
        return {"kind": "dict", "items": [{"key": encode_value(k, session_id), "value": encode_value(val, session_id)} for k, val in v.items()]}
    
    # Otherwise treat as Handle
    # Generate ID if not already tracked? 
    # In real implementation we might use id(v) but here we generate UUID
    # For prototype, we create a new handle every time it crosses boundary to be safe
    obj_id = f"h_{uuid.uuid4().hex[:8]}"
    store = get_session_store(session_id)
    store[obj_id] = v
    return {
        "kind": "handle",
        "id": obj_id,
        "type": type(v).__name__,
        "repr": repr(v),
        "session_id": session_id
    }

def decode_value(tnk_val, session_id):
    """Decode TnkValue to Python value."""
    kind = tnk_val.get("kind")
    if kind == "value":
        return tnk_val["value"]
    if kind == "handle":
        hid = tnk_val["id"]
        store = get_session_store(session_id)
        if hid not in store:
            raise KeyError(f"StaleHandle: {hid}")
        return store[hid]
    if kind == "list":
        return [decode_value(x, session_id) for x in tnk_val["items"]]
    if kind == "tuple":
        return tuple(decode_value(x, session_id) for x in tnk_val["items"])
    if kind == "dict":
        return {decode_value(x["key"], session_id): decode_value(x["value"], session_id) for x in tnk_val["items"]}
    raise ValueError(f"Unknown TnkValue kind: {kind}")

# --- Command Handlers ---

def handle_call_method(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    method_name = cmd["method"]
    args = [decode_value(a, session_id) for a in cmd["args"]]
    
    store = get_session_store(session_id)
    if target_id not in store:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})
    
    obj = store[target_id]
    if not hasattr(obj, method_name):
         return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": "AttributeError", "message": f"{type(obj)} has no attribute {method_name}"})
    
    func = getattr(obj, method_name)
    try:
        result = func(*args)
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e), "traceback": traceback.format_exc()})

def handle_get_attribute(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    attr_name = cmd["name"]
    
    if attr_name.startswith("_"):
        return make_response(cmd.get("req_id"), error={"code": "SecurityViolation", "message": "Access to private attributes is forbidden"})

    store = get_session_store(session_id)
    if target_id not in store:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})

    obj = store[target_id]
    try:
        result = getattr(obj, attr_name)
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})

def handle_get_item(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    key = decode_value(cmd["key"], session_id)
    
    store = get_session_store(session_id)
    if target_id not in store:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})
    
    obj = store[target_id]
    try:
        result = obj[key]
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})

def handle_slice(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    
    # decode start/stop/step
    # Spec says: kind:value(number|null) or kind:handle(int)
    def decode_slice_arg(arg):
        if arg["kind"] == "value":
            return arg["value"]
        elif arg["kind"] == "handle":
            val = decode_value(arg, session_id)
            if not isinstance(val, int):
                 raise TypeError("Slice argument from handle must be int")
            return val
        else:
            raise ValueError(f"Invalid slice arg kind: {arg['kind']}")

    try:
        start = decode_slice_arg(cmd["start"])
        stop = decode_slice_arg(cmd["stop"])
        step = decode_slice_arg(cmd["step"])
        
        if step == 0:
             return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": "ValueError", "message": "slice step cannot be zero"})

    except Exception as e:
         return make_response(cmd.get("req_id"), error={"code": "TypeMismatch", "message": str(e)})

    store = get_session_store(session_id)
    if target_id not in store:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})
    
    obj = store[target_id]
    try:
        # Create slice object
        sl = slice(start, stop, step)
        result = obj[sl]
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
         return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})

def handle_iter(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    
    store = get_session_store(session_id)
    if target_id not in store:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})
    
    obj = store[target_id]
    try:
        it = iter(obj)
        # Store iterator as a new handle
        it_id = f"it_{uuid.uuid4().hex[:8]}"
        store[it_id] = it
        
        return make_response(cmd.get("req_id"), value={
            "kind": "handle",
            "id": it_id,
            "type": type(it).__name__,
            "repr": repr(it),
            "session_id": session_id
        })
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})

def handle_iter_next_batch(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    batch_size = cmd["batch_size"]
    
    store = get_session_store(session_id)
    if target_id not in store:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})
    
    it = store[target_id]
    items = []
    done = False
    try:
        for _ in range(batch_size):
            item = next(it)
            items.append(encode_value(item, session_id))
    except StopIteration:
        done = True
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})
    
    return make_response(cmd.get("req_id"), value={"kind": "list", "items": items}, meta={"done": done})

def handle_delete(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    
    store = get_session_store(session_id)
    if target_id in store:
        del store[target_id]
    
    return make_response(cmd.get("req_id"), value={"kind": "value", "value": None})

def create_handle(v, session_id):
    obj_id = f"h_{uuid.uuid4().hex[:8]}"
    store = get_session_store(session_id)
    store[obj_id] = v
    return {
        "kind": "handle",
        "id": obj_id,
        "type": type(v).__name__,
        "repr": repr(v),
        "session_id": session_id
    }

# --- Debug Commands for Verification ---
def handle_debug_create_string(cmd):
    session_id = cmd["session_id"]
    val = cmd["value"]
    # Force handle creation for testing method calls on strings
    return make_response(cmd.get("req_id"), value=create_handle(val, session_id))

def handle_debug_eval(cmd):
    session_id = cmd["session_id"]
    code = cmd["code"]
    try:
        val = eval(code, {"__builtins__": {}}, {}) # Clean eval for test
        # Force handle creation so we can test iter/slice on it
        return make_response(cmd.get("req_id"), value=create_handle(val, session_id))
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "message": str(e)})

# --- Main Dispatch Loop ---

DISPATCHER = {
    "call_method": handle_call_method,
    "get_attribute": handle_get_attribute,
    "get_item": handle_get_item,
    "slice": handle_slice,
    "iter": handle_iter,
    "iter_next_batch": handle_iter_next_batch,
    "delete": handle_delete,
    "debug_create_string": handle_debug_create_string,
    "debug_eval": handle_debug_eval,
}

def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            cmd = json.loads(line)
            cmd_name = cmd.get("cmd")
            if cmd_name in DISPATCHER:
                resp = DISPATCHER[cmd_name](cmd)
            else:
                resp = make_response(cmd.get("req_id"), error={"code": "ProtocolError", "message": f"Unknown command {cmd_name}"})
        except json.JSONDecodeError:
             resp = make_response(None, error={"code": "ProtocolError", "message": "Invalid JSON"})
        except Exception as e:
             resp = make_response(None, error={"code": "WorkerCrash", "message": str(e), "traceback": traceback.format_exc()})
        
        print(json.dumps(resp))
        sys.stdout.flush()

if __name__ == "__main__":
    main()
