fn find_first_even(nums: &[serde_json::Value]) -> i64 {
    let mut found: Option<i64> = None;
    for n in nums.iter().cloned() {
        if (n % 2i64) == 0i64 {
            found = Some(n);
            break;
        } else {
            found = Some(-1i64);
        }
    }
    return found.unwrap();
}
fn sum_loop(limit: i64) -> i64 {
    let mut i: Option<i64> = None;
    let mut total: i64 = 0i64;
    for _loop_i in 0i64..limit {
        i = Some(_loop_i);
        total = (total + i.unwrap());
    }
    return i.unwrap();
}
fn main() {
    let result = std::panic::catch_unwind(|| {
    println!("{:?}", &find_first_even(&vec![1i64, 3i64, 4i64, 5i64, 6i64]));

    println!("{:?}", &find_first_even(&vec![1i64, 3i64, 5i64, 7i64]));

    println!("{:?}", &sum_loop(5i64));

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