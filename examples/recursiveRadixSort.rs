fn getOrder(num: i64) -> i64 {
    let digits: i64 = (num.to_string().len() as i64);
    return ((10 as f64).powf((digits - 1) as f64) as i64);
}
fn sortInOrder(lists: Vec<i64>, order: i64) -> Vec<i64> {
    if (order == 0) {
        return lists;
    }
    let mut number_list: Vec<Vec<i64>> = IntoIterator::into_iter(0..10).map(|value| vec![]).collect::<Vec<_>>();
    let n: i64 = 0;
    for n in lists {
        let idx: i64 = ((n % (order * 10)) / order);
        number_list[(idx as usize)].push(n);
    }
    let mut sorted_list: Vec<i64> = vec![];
    let i: i64 = 0;
    for i in 0..10 {
        sorted_list.extend(sortInOrder(number_list[(i as usize)].clone(), (order / 10).clone()));
    }
    return sorted_list;
}
fn recursiveRadixSort(lists: Vec<i64>) -> Vec<i64> {
    let max_value: i64 = lists.iter().max().cloned().unwrap();
    let mut sorted_list: Vec<i64> = sortInOrder(lists.clone(), getOrder(max_value).clone());
    return sorted_list;
}
fn user_main() -> () {
    let mut test_list: Vec<i64> = vec![170, 45, 75, 90, 802, 24, 2, 66];
    println!("{:?} {:?}", "元のリスト:".clone(), test_list.clone());
    let mut result: Vec<i64> = recursiveRadixSort(test_list.clone());
    println!("{:?} {:?}", "ソート後:".clone(), result.clone());
}
fn main() -> () {
    user_main();
}