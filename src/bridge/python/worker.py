"""
Tsuchinoko V1.7.0 Python Worker
===============================
Standard RPC worker for remote object handling.
Supports multiple sessions, robust error handling, and batched iteration.
"""

import sys
import json
import uuid
import traceback
import importlib

# --- Diagnostics (V1.7.0 Robustness) ---
try:
    print(f"[Worker] Initializing... Executable: {sys.executable}", file=sys.stderr)
    print(f"[Worker] Version: {sys.version.split()[0]}", file=sys.stderr)
    print(f"[Worker] Path: {sys.path}", file=sys.stderr)
except Exception as e:
    print(f"[Worker] Diagnostic failed: {e}", file=sys.stderr)
# ---------------------------------------

# --- Global State ---
# _SESSIONS[session_id] = { "objects": {id: obj}, "modules": {name: module} }
_SESSIONS = {}

# --- Security Policy (V1.7.0) ---
FORBIDDEN_CALLS = {"eval", "exec", "globals", "locals"}

def is_forbidden_name(name):
    return name in FORBIDDEN_CALLS

def is_forbidden_target(target_str):
    # "builtins.eval" -> "eval"
    parts = target_str.split(".")
    tail = parts[-1] if parts else target_str
    return is_forbidden_name(tail)

def get_session(session_id):
    if session_id not in _SESSIONS:
        _SESSIONS[session_id] = {"objects": {}, "modules": {}}
    return _SESSIONS[session_id]

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
        # Bool must come before int check because isinstance(True, int) is True
        return {"kind": "value", "value": v}
    if isinstance(v, (int, float)):
        return {"kind": "value", "value": v}
    # Scalar-like objects (e.g., numpy scalars) -> convert via item() generically
    try:
        if hasattr(v, "item") and callable(getattr(v, "item")):
            return encode_value(v.item(), session_id)
    except Exception:
        pass
    if isinstance(v, str):
        return {"kind": "value", "value": v}
    if isinstance(v, (list, tuple)):
        # Recursively encode list/tuple. Note: numpy arrays etc should be handles?
        # Maximum A spec says primitives. 
        # For simplicity in V1.7.0, lists of primitives are lists, 
        # but lists of complex objects... technically TnkValue can contain Handles.
        kind = "list" if isinstance(v, list) else "tuple"
        return {"kind": kind, "items": [encode_value(x, session_id) for x in v]}
    if isinstance(v, dict):
        return {"kind": "dict", "items": [{"key": encode_value(k, session_id), "value": encode_value(val, session_id)} for k, val in v.items()]}
    
    # Everything else is a Handle
    # Check if object already has ID? (Not strictly required for Opaque Handle, but nice for equality)
    # For V1.7.0, we just create a new handle.
    obj_id = f"h_{uuid.uuid4().hex[:16]}"
    session = get_session(session_id)
    session["objects"][obj_id] = v
    
    type_name = type(v).__name__
    try:
        repr_str = repr(v)
        if len(repr_str) > 200:
            repr_str = repr_str[:197] + "..."
    except:
        repr_str = f"<{type_name} object>"

    try:
        str_str = str(v)
        if len(str_str) > 200:
            str_str = str_str[:197] + "..."
    except:
        str_str = repr_str

    return {
        "kind": "handle",
        "id": obj_id,
        "type": type_name,
        "repr": repr_str,
        "str": str_str,
        "session_id": session_id
    }

def decode_value(tnk_val, session_id):
    """Decode TnkValue to Python value."""
    kind = tnk_val.get("kind")
    if kind == "value":
        return tnk_val["value"]
    if kind == "handle":
        hid = tnk_val["id"]
        # Allow cross-session handles? No, spec says session_id must match? 
        # Actually spec says: "handle.session_id is for verification".
        # If handle points to another session, we probably can't resolve it unless we look globally.
        # But for strictness:
        h_sid = tnk_val.get("session_id")
        if h_sid and h_sid != session_id:
             # If we want to support sharing, we'd need global store. 
             # For V1.7.0, let's look in the requesting session.
             pass 

        session = get_session(session_id) # Using request's session scope
        if hid not in session["objects"]:
             # Check if it was session mismatch
             raise KeyError(f"StaleHandle: {hid} (Session: {session_id})")
        return session["objects"][hid]
    
    if kind == "list":
        return [decode_value(x, session_id) for x in tnk_val["items"]]
    if kind == "tuple":
        return tuple(decode_value(x, session_id) for x in tnk_val["items"])
    if kind == "dict":
        return {decode_value(x["key"], session_id): decode_value(x["value"], session_id) for x in tnk_val["items"]}
    
    raise ValueError(f"Unknown TnkValue kind: {kind}")

# --- Logic ---

def resolve_callable(target_str, session_id):
    """Resolve 'numpy.array' -> function object."""
    parts = target_str.split(".")
    if not parts:
        raise ValueError("Empty target")
    
    # Try dynamic import structure
    # 1. Try importing the first part as module
    module_name = parts[0]
    session = get_session(session_id)
    
    # Cache in session modules? Or global?
    # Python modules are global sys.modules, but we can cache ref in session if needed.
    
    try:
        current_obj = importlib.import_module(module_name)
    except ImportError:
        # Maybe it's a builtin?
        import builtins
        if hasattr(builtins, module_name):
            current_obj = getattr(builtins, module_name)
        else:
             raise
             
    # Traverse the rest
    for part in parts[1:]:
        current_obj = getattr(current_obj, part)
        
    return current_obj

def resolve_target(target, session_id):
    """Resolve target (Handle ID or Module definition) to a Python object."""
    if isinstance(target, dict) and target.get("kind") == "module":
        # Target is a module: {"kind": "module", "module": "cv2"}
        module_name = target["module"]
        return resolve_callable(module_name, session_id)
    
    # Otherwise, assume target is a Handle ID (string)
    if not isinstance(target, str):
         raise ValueError(f"Invalid target format: {target}")
         
    session = get_session(session_id)
    if target not in session["objects"]:
        raise ValueError(f"StaleHandle: {target}")
        
    return session["objects"][target]


# --- Command Handlers ---

def handle_call_function(cmd):
    """NEW: Call a global function or static method by string path. 
    e.g. target="numpy.array", args=[...], kwargs={...}
    """
    session_id = cmd["session_id"]
    target_str = cmd["target"]
    args = [decode_value(a, session_id) for a in cmd["args"]]
    kwargs = {k: decode_value(v, session_id) for k, v in (cmd.get("kwargs") or {}).items()}

    if is_forbidden_target(target_str):
        return make_response(cmd.get("req_id"), error={"code": "SecurityViolation", "message": f"Forbidden function call: {target_str}"})
    
    func = None
    try:
        func = resolve_callable(target_str, session_id)
    except ImportError:
         return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": "ImportError", "message": f"Module implementation not found: {target_str}"})
    except AttributeError:
         return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": "AttributeError", "message": f"Attribute not found: {target_str}"})
    except Exception as e:
         return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e), "traceback": traceback.format_exc()})

    try:
        result = func(*args, **kwargs)
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e), "traceback": traceback.format_exc()})

def handle_call_method(cmd):
    session_id = cmd["session_id"]
    target = cmd["target"]
    method_name = cmd["method"]
    args = [decode_value(a, session_id) for a in cmd["args"]]
    kwargs = {k: decode_value(v, session_id) for k, v in (cmd.get("kwargs") or {}).items()}

    if is_forbidden_name(method_name):
        return make_response(cmd.get("req_id"), error={"code": "SecurityViolation", "message": f"Forbidden method call: {method_name}"})
    
    try:
        obj = resolve_target(target, session_id)
    except ValueError as e:
        if "StaleHandle" in str(e):
             return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": str(e)})
        return make_response(cmd.get("req_id"), error={"code": "ProtocolError", "message": str(e)})

    if not hasattr(obj, method_name):
         return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": "AttributeError", "message": f"{type(obj)} has no attribute {method_name}"})
    
    func = getattr(obj, method_name)
    try:
        result = func(*args, **kwargs)
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e), "traceback": traceback.format_exc()})

def handle_get_attribute(cmd):
    session_id = cmd["session_id"]
    target = cmd["target"]
    attr_name = cmd["name"]
    
    if attr_name.startswith("_"):
        return make_response(cmd.get("req_id"), error={"code": "SecurityViolation", "message": "Access to private attributes is forbidden"})
    if is_forbidden_name(attr_name):
        return make_response(cmd.get("req_id"), error={"code": "SecurityViolation", "message": f"Forbidden attribute access: {attr_name}"})

    try:
        obj = resolve_target(target, session_id)
    except ValueError as e:
        if "StaleHandle" in str(e):
             return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": str(e)})
        return make_response(cmd.get("req_id"), error={"code": "ProtocolError", "message": str(e)})

    try:
        result = getattr(obj, attr_name)
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})

def handle_get_item(cmd):
    session_id = cmd["session_id"]
    target = cmd["target"]
    key = decode_value(cmd["key"], session_id)
    
    try:
        obj = resolve_target(target, session_id)
    except ValueError as e:
        if "StaleHandle" in str(e):
             return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": str(e)})
        return make_response(cmd.get("req_id"), error={"code": "ProtocolError", "message": str(e)})

    try:
        result = obj[key]
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})

def handle_slice(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    
    def decode_slice_arg(arg):
        if arg["kind"] == "value":
            return arg["value"]
        elif arg["kind"] == "handle":
            val = decode_value(arg, session_id)
            try:
                # Be a bit more permissive, try supporting types that execute as index
                # But spec says strictly check/convert
                return int(val) 
            except:
                 raise TypeError(f"Slice argument handle must resolve to int, got {type(val)}")
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

    session = get_session(session_id)
    if target_id not in session["objects"]:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})
    
    obj = session["objects"][target_id]
    try:
        sl = slice(start, stop, step)
        result = obj[sl]
        return make_response(cmd.get("req_id"), value=encode_value(result, session_id))
    except Exception as e:
         return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})

def handle_iter(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    
    session = get_session(session_id)
    if target_id not in session["objects"]:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})
    
    obj = session["objects"][target_id]
    try:
        it = iter(obj)
        it_id = f"it_{uuid.uuid4().hex[:16]}"
        session["objects"][it_id] = it
        
        return make_response(cmd.get("req_id"), value={
            "kind": "handle",
            "id": it_id,
            "type": type(it).__name__,
            "repr": repr(it),
            "str": str(it),
            "session_id": session_id
        })
    except Exception as e:
        return make_response(cmd.get("req_id"), error={"code": "PythonException", "py_type": type(e).__name__, "message": str(e)})

def handle_iter_next_batch(cmd):
    session_id = cmd["session_id"]
    target_id = cmd["target"]
    batch_size = cmd.get("batch_size", 1000)
    
    session = get_session(session_id)
    if target_id not in session["objects"]:
        return make_response(cmd.get("req_id"), error={"code": "StaleHandle", "message": f"Handle {target_id} not found"})
    
    it = session["objects"][target_id]
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
    
    session = get_session(session_id)
    if target_id in session["objects"]:
        del session["objects"][target_id]
    
    return make_response(cmd.get("req_id"), value={"kind": "value", "value": None})

# --- Main Dispatch ---

DISPATCHER = {
    "call_function": handle_call_function, # Added for bootstrapping
    "call_method": handle_call_method,
    "get_attribute": handle_get_attribute,
    "get_item": handle_get_item,
    "slice": handle_slice,
    "iter": handle_iter,
    "iter_next_batch": handle_iter_next_batch,
    "delete": handle_delete,
}

def main():
    # Unbuffered stdin/stdout is handled by parent, but we can flush manually
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
