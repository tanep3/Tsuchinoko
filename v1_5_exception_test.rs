fn get_zero() -> i64 {
    // ""Helper to get zero at runtime""
    return 0i64;
}
fn test_basic_try_except() -> i64 {
    // ""Basic try-except with panic""
    let mut result: i64 = 0i64;
    let mut zero: Option<i64> = None;
    let mut x: Option<i64> = None;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        zero = Some(get_zero());
        x = Some((1i64 / zero.clone().unwrap()));
    })) {
        Ok(__val) => __val,
        Err(_) => {
            result = 1i64;
        }
    }

    return result;
}
fn test_try_success() -> i64 {
    // ""Test try block without exception""
    let mut result: i64 = 0i64;
    let mut x: Option<i64> = None;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        x = Some(10i64);
        result = x.clone().unwrap();
    })) {
        Ok(__val) => __val,
        Err(_) => {
            result = -1i64;
        }
    }

    return result;
}
fn test_except_type() -> i64 {
    // ""EX-001: except ValueError:""
    let mut result: i64 = 0i64;
    let mut zero: Option<i64> = None;
    let mut x: Option<i64> = None;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        zero = Some(get_zero());
        x = Some((1i64 / zero.clone().unwrap()));
    })) {
        Ok(__val) => __val,
        Err(_) => {
            result = 2i64;
        }
    }

    return result;
}
fn test_multiple_except_types() -> i64 {
    // ""EX-002: except (TypeError, ValueError):""
    let mut result: i64 = 0i64;
    let mut zero: Option<i64> = None;
    let mut x: Option<i64> = None;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        zero = Some(get_zero());
        x = Some((1i64 / zero.clone().unwrap()));
    })) {
        Ok(__val) => __val,
        Err(_) => {
            result = 3i64;
        }
    }

    return result;
}
fn test_except_as() -> i64 {
    // ""EX-003: except ValueError as e:""
    let mut result: i64 = 0i64;
    let mut zero: Option<i64> = None;
    let mut x: Option<i64> = None;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        zero = Some(get_zero());
        x = Some((1i64 / zero.clone().unwrap()));
    })) {
        Ok(__val) => __val,
        Err(__exc) => {
            let e = TsuchinokoError::new("Exception", &format!("{:?}", __exc), None);
            result = 4i64;
        }
    }

    return result;
}
fn test_finally() -> i64 {
    // ""EX-004: finally block""
    let mut result: i64 = 0i64;
    let mut x: Option<i64> = None;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        x = Some(10i64);
        result = x.clone().unwrap();
    })) {
        Ok(__val) => __val,
        Err(_) => {
            result = -1i64;
        }
    }
    // finally block
    result = (result + 100i64);

    return result;
}
fn test_finally_with_exception() -> i64 {
    // ""EX-004: finally executes even on exception""
    let mut result: i64 = 0i64;
    let mut zero: Option<i64> = None;
    let mut x: Option<i64> = None;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        zero = Some(get_zero());
        x = Some((1i64 / zero.clone().unwrap()));
    })) {
        Ok(__val) => __val,
        Err(_) => {
            result = 50i64;
        }
    }
    // finally block
    result = (result + 100i64);

    return result;
}
fn main_py() -> () {
    println!("{:?}", &test_basic_try_except());

    println!("{:?}", &test_try_success());

    println!("{:?}", &test_except_type());

    println!("{:?}", &test_multiple_except_types());

    println!("{:?}", &test_except_as());

    println!("{:?}", &test_finally());

    println!("{:?}", &test_finally_with_exception());

}
fn main() {
    let result = std::panic::catch_unwind(|| {
    main_py();

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