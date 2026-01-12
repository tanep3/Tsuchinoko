from typing import Any
import json as j

def main():
    # 2. Module assignment to Any
    # This should verify py_bridge.get("j")
    # Using mod_chk to avoid Rust 'mod' keyword collision
    mod_chk: Any = j
    
    # 3. Method call on ModuleRef
    # j.dumps returns String (TnkValue)
    data = [1, 2, 3]
    s: Any = j.dumps(data)
    
    # 4. Verify fluent syntax usage in generated code
    # This should generate: py_bridge.import("json", "j"); 
    # and py_bridge.get("j").call_method("dumps", ...)
    print(s)

if __name__ == "__main__":
    main()
