#[derive(Clone)]
struct Counter {
    count: i64,
}
impl Counter {
    fn increment(&mut self, ) -> i64 {
        self.count = (self.count + 1i64);
        return self.count;
    }
}

fn main() -> () {
    let mut c: Counter = Counter { count: 0i64 };
    println!("{:?}", &c.increment());
}