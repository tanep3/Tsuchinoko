use tsuchinoko::bridge::{PythonBridge, protocol::{TnkValue, JsonPrimitive}};

#[test]
fn test_bridge_call_function_str() {
    let mut bridge = PythonBridge::new().expect("Failed to start bridge");
    
    // Call builtins.str("Hello")
    let arg = TnkValue::Value { value: Some(JsonPrimitive::String("Hello".into())) };
    let result = bridge.call_function("builtins.str", vec![arg]).expect("Call failed");
    
    match result {
        TnkValue::Handle { type_, repr, .. } => {
            assert_eq!(type_, "str");
            assert_eq!(repr, "'Hello'");
        },
        _ => panic!("Expected Handle, got {:?}", result),
    }
}

#[test]
fn test_bridge_math_sqrt() {
    let mut bridge = PythonBridge::new().expect("Failed to start bridge");
    
    let arg = TnkValue::Value { value: Some(JsonPrimitive::Number(9.0)) };
    let result = bridge.call_function("math.sqrt", vec![arg]).expect("Call failed");
    
    match result {
        TnkValue::Value { value: Some(JsonPrimitive::Number(n)) } => {
            assert_eq!(n, 3.0);
        },
        _ => panic!("Expected Number, got {:?}", result),
    }
}

#[test]
fn test_bridge_list_slice() {
    let mut bridge = PythonBridge::new().expect("Failed to start bridge");
    
    // Create list [1, 2, 3] via builtins type? OR just manual construction not easy without eval.
    // Use json.loads?
    // Let's use `builtins.list` on tuple `(1, 2, 3)`? 
    // bridge serializes vec as list/tuple.
    
    let args_vec = vec![
        TnkValue::Value { value: Some(JsonPrimitive::Number(1.0)) },
        TnkValue::Value { value: Some(JsonPrimitive::Number(2.0)) },
        TnkValue::Value { value: Some(JsonPrimitive::Number(3.0)) },
    ];
    let arg_tuple = TnkValue::Tuple { items: args_vec }; // Serialize as tuple
    
    // builtins.list((1,2,3))
    let list_handle = bridge.call_function("builtins.list", vec![arg_tuple]).expect("Create list failed");
    let h_id = match list_handle {
        TnkValue::Handle { id, .. } => id,
        _ => panic!("Expected handle"),
    };
    
    // Slice [0:2:1]
    let start = TnkValue::Value { value: Some(JsonPrimitive::Number(0.0)) };
    let stop = TnkValue::Value { value: Some(JsonPrimitive::Number(2.0)) };
    let step = TnkValue::Value { value: Some(JsonPrimitive::Number(1.0)) };
    
    let slice_result = bridge.slice(&h_id, start, stop, step).expect("Slice failed");
    
    match slice_result {
        TnkValue::List { items } => {
            assert_eq!(items.len(), 2);
        },
        TnkValue::Handle { type_, .. } if type_ == "list" => {
            // Depending on complexity, it might return Handle for list.
            // V1.7.0 Worker `encode_value`: "Recursively encode list/tuple" IF it is list/tuple.
            // `obj[sl]` returns a NEW list. `encode_value` sees a list. It encodes as `kind: "list"`.
            // So we expect List.
             panic!("Expected List value, got Handle (maybe okay but expected value for simple list)");
        },
        _ => panic!("Expected List, got {:?}", slice_result),
    }
}
