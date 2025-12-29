fn greet(name: &str, greeting: &str) -> String {
    return format!("{}, {}!", greeting, name);
}
fn add(a: i64, b: i64, c: i64) -> i64 {
    let result: i64 = (a + b) + c;
    return result;
}
fn main() -> () {
    let msg1: String = greet(&"World", &"Hello");
    let msg2: String = greet(&"Tane", &"Hi");
    let msg3: String = greet(&"User", &"Welcome");
    let msg4: String = greet(&"Friend", &"Hey");
    let sum1: i64 = add(1i64, 10i64, 20i64);
    let sum2: i64 = add(1i64, 2i64, 20i64);
    let sum3: i64 = add(1i64, 2i64, 3i64);
    let sum4: i64 = add(5i64, 15i64, 25i64);
    println!("{:?}", &msg1);
    println!("{:?}", &msg2);
    println!("{:?}", &msg3);
    println!("{:?}", &msg4);
    println!("{:?}", &sum1);
    println!("{:?}", &sum2);
    println!("{:?}", &sum3);
    println!("{:?}", &sum4);
}