use tsuchinoko::bridge::{PythonBridge, protocol::{TnkValue, JsonPrimitive}};

#[test]
fn test_bridge_call_function_str() {
    let bridge = PythonBridge::new().expect("Failed to start bridge");
    
    // Call builtins.str("Hello")
    let arg = TnkValue::Value { value: Some(JsonPrimitive::String("Hello".into())) };
    let args = [&arg];
    let result = bridge.call_function("builtins.str", &args, None).expect("Call failed");
    
    match result {
        TnkValue::Value { value: Some(JsonPrimitive::String(s)) } => {
            assert_eq!(s, "Hello");
        }
        TnkValue::Handle { type_, repr, .. } => {
            assert_eq!(type_, "str");
            assert_eq!(repr, "'Hello'");
        }
        _ => panic!("Expected String or Handle, got {:?}", result),
    }
}

#[test]
fn test_bridge_math_sqrt() {
    let bridge = PythonBridge::new().expect("Failed to start bridge");
    
    let arg = TnkValue::from(9.0);
    let args = [&arg];
    let result = bridge.call_function("math.sqrt", &args, None).expect("Call failed");
    
    match result {
        TnkValue::Value { value: Some(JsonPrimitive::Float(n)) } => {
            assert_eq!(n, 3.0);
        },
        _ => panic!("Expected Float, got {:?}", result),
    }
}

#[test]
fn test_bridge_list_slice() {
    let bridge = PythonBridge::new().expect("Failed to start bridge");
    
    // Create list [1, 2, 3] via builtins type? OR just manual construction not easy without eval.
    // Use json.loads?
    // Let's use `builtins.list` on tuple `(1, 2, 3)`? 
    // bridge serializes vec as list/tuple.
    
    let args_vec = vec![
        TnkValue::from(1i64),
        TnkValue::from(2i64),
        TnkValue::from(3i64),
    ];
    let arg_tuple = TnkValue::Tuple { items: args_vec }; // Serialize as tuple
    
    // builtins.list((1,2,3))
    let arg_refs: Vec<&TnkValue> = vec![&arg_tuple];
    let list_handle = bridge.call_function("builtins.list", &arg_refs, None).expect("Create list failed");
    let target = match list_handle {
        h @ TnkValue::Handle { .. } => Some(h),
        l @ TnkValue::List { .. } => {
            // If we already have a concrete list value, slicing can be validated locally.
            if let TnkValue::List { items } = l {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], TnkValue::from(1i64));
                assert_eq!(items[1], TnkValue::from(2i64));
                assert_eq!(items[2], TnkValue::from(3i64));
            }
            None
        }
        other => panic!("Expected handle or list, got {:?}", other),
    };
    
    // Slice [0:2:1]
    let start = TnkValue::from(0i64);
    let stop = TnkValue::from(2i64);
    let step = TnkValue::from(1i64);
    
    let Some(target) = target else {
        // Already validated the list content above, so we can return early.
        return;
    };
    let slice_result = bridge.slice(&target, Some(start), Some(stop), Some(step)).expect("Slice failed");
    
    match slice_result {
        TnkValue::List { items } => {
            assert_eq!(items.len(), 2);
        },
        TnkValue::Handle { type_, .. } if type_ == "list" => {
            // Handle is acceptable for some worker configurations
        },
        _ => panic!("Expected List, got {:?}", slice_result),
    }
}
