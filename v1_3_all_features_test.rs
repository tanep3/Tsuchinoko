fn demo_not_in() -> () {
    // ""not in 演算子のデモ""
    let mut nums: Vec<i64> = vec![1i64, 2i64, 3i64, 4i64, 5i64];
    if !nums.contains(&10i64) {
        println!("{}", "10 is not in the list");

    }
    if !nums.contains(&3i64) {
        println!("{}", "This should not print");

    }
}
fn demo_bitwise() -> () {
    // ""ビット演算子のデモ""
    let a: i64 = 0b1100;
    let b: i64 = 0b1010;
    println!("{:?}", &(a & b));

    println!("{:?}", &(a | b));

    println!("{:?}", &(a ^ b));

    println!("{:?}", &(!a));

    println!("{:?}", &(a << 2i64));

    println!("{:?}", &(a >> 2i64));

}
fn demo_aug_assign() -> () {
    // ""累算代入演算子のデモ""
    let mut x: i64 = 8i64;
    x &= 4i64;
    println!("{:?}", &x);

    let mut y: i64 = 8i64;
    y |= 4i64;
    println!("{:?}", &y);

    let mut z: i64 = 2i64;
    z = (z as i64).pow((10i64) as u32);
    println!("{:?}", &z);

}
fn demo_enumerate_zip() -> () {
    // ""enumerate と zip のデモ""
    let mut fruits: Vec<String> = vec!["apple".to_string(), "banana".to_string(), "cherry".to_string()];
    let mut prices: Vec<i64> = vec![100i64, 200i64, 150i64];
    for (i, fruit) in fruits.iter().enumerate().map(|(i, x)| (i as i64, x.clone())) {
        println!("{:?}", &format!("{:?}: {:?}", i, fruit));

    }
    for (i, fruit) in fruits.iter().enumerate().map(|(i, x)| (i as i64 + 1, x.clone())) {
        println!("{:?}", &format!("#{:?} {:?}", i, fruit));

    }
    for (fruit, price) in fruits.iter().zip(prices.iter()).map(|(x, y)| (x.clone(), y.clone())) {
        println!("{:?}", &format!("{:?} costs {:?}", fruit, price));

    }
}
fn demo_sorted_reversed() -> () {
    // ""sorted と reversed のデモ""
    let mut nums: Vec<i64> = vec![3i64, 1i64, 4i64, 1i64, 5i64, 9i64, 2i64, 6i64];
    println!("{:?}", &{ let mut v = nums.to_vec(); v.sort(); v });

    println!("{:?}", &{ let mut v = nums.to_vec(); v.sort_by(|a, b| b.cmp(a)); v });

    for x in nums.iter().rev().cloned() {
        println!("{:?}", &x);

    }
}
fn demo_sum_all_any() -> () {
    // ""sum, all, any のデモ""
    let mut nums: Vec<i64> = vec![1i64, 2i64, 3i64, 4i64, 5i64];
    println!("{:?}", &nums.iter().sum::<i64>());

    println!("{:?}", &nums.iter().sum::<i64>() + 100i64);

    println!("{:?}", &nums.iter().cloned().map(|x| (x > 0i64)).collect::<Vec<_>>().iter().all(|x| *x));

    println!("{:?}", &nums.iter().cloned().map(|x| (x > 10i64)).collect::<Vec<_>>().iter().any(|x| *x));

}
fn demo_map_filter() -> () {
    // ""map と filter のデモ""
    let mut nums: Vec<i64> = vec![1i64, 2i64, 3i64, 4i64, 5i64];
    let mut doubled: Vec<i64> = nums.iter().map(move |x| {
        return (x * 2i64);

}).collect();
    println!("{:?}", &doubled);

    let mut evens: Vec<i64> = nums.iter().cloned().filter(|&x| x % 2i64 == 0i64).collect();
    println!("{:?}", &evens);

}
fn demo_assert(x: i64) -> i64 {
    // ""assert のデモ""
    assert!((x > 0i64), "x must be positive");
    return (x * 2i64);
}
fn demo_list_methods() -> () {
    // ""リストメソッドのデモ""
    let mut nums: Vec<i64> = vec![3i64, 1i64, 4i64, 1i64, 5i64, 9i64, 2i64, 6i64, 5i64, 3i64, 5i64];
    println!("{:?}", &(nums.iter().filter(|e| **e == 5i64).count() as i64));

    println!("{:?}", &(nums.iter().position(|e| *e == 9i64).unwrap() as i64));

    let mut sorted_nums: Vec<i64> = nums.to_vec();
    sorted_nums.sort();

    println!("{:?}", &sorted_nums);

    let mut reversed_nums: Vec<i64> = nums.to_vec();
    reversed_nums.reverse();

    println!("{:?}", &reversed_nums);

}
fn demo_dict_comp() -> () {
    // ""辞書内包表記のデモ""
    let mut nums: Vec<i64> = vec![1i64, 2i64, 3i64, 4i64, 5i64];
    let squares: std::collections::HashMap<i64, i64> = nums.iter().cloned().map(|x| (x, (x * x))).collect::<std::collections::HashMap<_, _>>();
    println!("{:?}", &squares);

    let even_squares: std::collections::HashMap<i64, i64> = nums.iter().cloned().filter(|&x| ((x % 2i64) == 0i64)).map(|x| (x, (x * x))).collect::<std::collections::HashMap<_, _>>();
    println!("{:?}", &even_squares);

}
fn demo_multi_assign() -> () {
    // ""多重代入のデモ""
    let (a, b, c) = (1i64, 2i64, 3i64);
    println!("{:?} {:?} {:?}", &a, &b, &c);

    let mut x: i64 = 10i64;
    let mut y: i64 = 20i64;
    (x, y) = (y, x);
    println!("{:?} {:?}", &x, &y);

    let mut fib_a: i64 = 0i64;
    let mut fib_b: i64 = 1i64;
    let mut result: Vec<i64> = vec![];
    for _ in 0i64..10i64 {
        result.push(fib_a);

        (fib_a, fib_b) = (fib_b, (fib_a + fib_b));
    }
    println!("{:?}", &result);

}
fn main() {
    let result = std::panic::catch_unwind(|| {
    println!("{}", "=== V1.3.0 Integration Test ===");

    println!("{}", "\n--- not in ---");

    demo_not_in();

    println!("{}", "\n--- bitwise ---");

    demo_bitwise();

    println!("{}", "\n--- augmented assign ---");

    demo_aug_assign();

    println!("{}", "\n--- enumerate/zip ---");

    demo_enumerate_zip();

    println!("{}", "\n--- sorted/reversed ---");

    demo_sorted_reversed();

    println!("{}", "\n--- sum/all/any ---");

    demo_sum_all_any();

    println!("{}", "\n--- map/filter ---");

    demo_map_filter();

    println!("{}", "\n--- assert ---");

    println!("{:?}", &demo_assert(5i64));

    println!("{}", "\n--- list methods ---");

    demo_list_methods();

    println!("{}", "\n--- dict comprehension ---");

    demo_dict_comp();

    println!("{}", "\n--- multi assign ---");

    demo_multi_assign();

    println!("{}", "\n=== All tests completed! ===");

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