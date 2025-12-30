fn sum_all(values: &[i64]) -> i64 {
    let mut total: i64 = 0i64;
    for v in values.iter().cloned() {
        total += v;
    }
    return total;
}
fn test_varargs() -> () {
    let result: i64 = sum_all(&1i64);
    println!("{:?}", &format!("sum_all(1,2,3,4,5) = {}", result));

}
fn main() -> () {
    println!("{:?}", "=== V1.2.0 Simple Test ===");

    test_varargs();

    println!("{:?}", "=== Done ===");

}