use tsuchinoko::bridge::PythonBridge;

fn test_numpy_mean(py_bridge: &mut tsuchinoko::bridge::PythonBridge, values: &[i64]) -> f64 {
    // ""numpy.mean を使って平均を計算""
    return {
let _arg_0 = values;
    py_bridge.call_json::<serde_json::Value>("numpy.mean", &[serde_json::json!(_arg_0)]).unwrap()
}.as_f64().unwrap();
}
fn main() {
    let mut py_bridge = tsuchinoko::bridge::PythonBridge::new()
        .expect("Failed to start Python worker");
    
        println!("{}", "=== NumPy Resident Test ===");

        let result: f64 = test_numpy_mean(&mut py_bridge, &vec![1i64, 2i64, 3i64, 4i64, 5i64]);
        println!("{:?}", &format!("mean([1,2,3,4,5]) = {:?}", result));

        println!("{}", "=== Done ===");
}

// Note: This code uses the PythonBridge for calling Python libraries.
// Make sure Python is installed and the required libraries are available.
// The Python worker process will be started automatically.
