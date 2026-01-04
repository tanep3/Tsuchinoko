fn test_dict_comp_basic(items: &[i64]) -> std::collections::HashMap<i64, i64> {
    // ""基本的な辞書内包表記""
    return items.iter().cloned().map(|i| (i, (i * 2i64))).collect::<std::collections::HashMap<_, _>>();
}
fn test_dict_comp_with_condition(items: &[i64]) -> std::collections::HashMap<i64, i64> {
    // ""条件付き辞書内包表記""
    return items.iter().cloned().filter(|&i| (i > 0i64)).map(|i| (i, (i * i))).collect::<std::collections::HashMap<_, _>>();
}
fn main() {
    let mut nums: Vec<i64> = vec![1i64, 2i64, 3i64, 4i64, 5i64];
    let result1: std::collections::HashMap<i64, i64> = test_dict_comp_basic(&nums);
    println!("{:?}", &result1);

    let mut mixed: Vec<i64> = vec![-2i64, -1i64, 0i64, 1i64, 2i64, 3i64];
    let result2: std::collections::HashMap<i64, i64> = test_dict_comp_with_condition(&mixed);
    println!("{:?}", &result2);

}