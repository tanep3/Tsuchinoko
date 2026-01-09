#[derive(Clone)]
struct Animal {
    name: String,
}
impl Animal {
    fn speak(&self, ) -> String {
        return "...";
    }
}

#[derive(Clone)]
struct Dog {
    base: Animal,
    breed: String,
}
impl Dog {
    fn speak(&self, ) -> String {
        return format!("{:?} says Woof!", self.name);
    }
}

fn program_start() -> () {
    let mut dog: Dog = Dog { base: "Rex".to_string(), breed: "Labrador".to_string() };
    println!("{:?}", &dog.speak());

}
fn main() {
    let result = std::panic::catch_unwind(|| {
    program_start();

    });
    if let Err(e) = result {
        let msg = if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        eprintln!("InternalError: {}", msg);
        std::process::exit(1);
    }
}