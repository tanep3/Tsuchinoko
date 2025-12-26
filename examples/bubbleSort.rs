fn bubbleSort(lists: Vec<i64>) -> (Vec<i64>, i64) {
    let mut sorted_list: Vec<i64> = lists.clone();
    let list_length: i64 = (sorted_list.len() as i64);
    let i: i64 = 0;
    let j: i64 = 0;
    for i in 0..list_length {
        for j in 0..((list_length - i) - 1) {
            if (sorted_list[(j as usize)] > sorted_list[((j + 1) as usize)]) {
                let temp: i64 = sorted_list[(j as usize)];
                sorted_list[(j as usize)] = sorted_list[((j + 1) as usize)];
                sorted_list[((j + 1) as usize)] = temp;
            }
        }
    }
    return (sorted_list, list_length);
}
fn program_start() -> () {
    let mut test_list: Vec<i64> = vec![64, 34, 25, 12, 22, 11, 90];
    println!("{:?} {:?}", "元のリスト:", test_list);
    let (sorted_lists, length) : (_, _) = bubbleSort(test_list);
    println!("{:?} {:?}", "ソート後のリスト:", sorted_lists);
}
fn main() -> () {
    program_start();
}