type ConditionFunction = Box<dyn Fn(i64, i64) -> bool + Send + Sync>;
#[derive(Clone, Debug)]
struct Condition {
    condition_function: ConditionFunction,
}
impl Condition {
    fn check(&self, num: i64, key_num: i64) -> bool {
        return self.condition_function(num, key_num);
    }
}

#[derive(Clone, Debug)]
struct Numbers {
    numbers: std::collections::HashMap<i64, String>,
}
impl Numbers {
    fn add(&self, key_num: i64, name: String) -> _ {
        if self.numbers.contains(&key_num) {

        }
        let new_numbers: _ = self.numbers.into_iter().collect::<std::collections::HashMap<_, _>>();
        new_numbers[key_num as usize] = name;
        return Numbers { numbers: new_numbers.clone() };
    }
    fn items(&self, ) -> Vec<(i64, String)> {
        return self.numbers.iter().to_vec();
    }
}

#[derive(Clone, Debug)]
struct FizzBuzz {
    condition: Condition,
    numbers: Numbers,
}
impl FizzBuzz {
    fn get_string(&self, num: i64) -> String {
        return self.numbers.iter().filter(|(key_num, name)| self.condition.check(num, key_num)).map(|(key_num, name)| name).collect::<Vec<_>>().iter().map(move |x| -> String {
                x.to_string()

}).collect::<Vec<String>>().join("");
    }
}

fn weekday_condition(num: i64, key_num: i64) -> bool {
    return ((num % key_num) == 0i64);
}
fn weekend_condition(num: i64, key_num: i64) -> bool {
    return (((num % 2i64) == 1i64) && ((num % key_num) == 0i64));
}
fn main() -> () {
    let mut weekends: Vec<_> = (1i64..32i64).filter(|d| ((((d - 1i64) % 7i64) == 0i64) || ((d % 7i64) == 0i64))).map(|d| d).collect::<Vec<_>>();
    let fizzbuzz_numbers: _ = Numbers { numbers: std::collections::HashMap::from([(3i64, "Fizz".to_string()), (5i64, "Buzz".to_string()), (7i64, "Lazz".to_string()), (11i64, "Pozz".to_string())]).clone() };
    let mut fizzbuzzs: Vec<_> = vec![FizzBuzz { condition: Condition { condition_function: weekday_condition }.clone(), numbers: fizzbuzz_numbers.clone() }, FizzBuzz { condition: Condition { condition_function: weekend_condition }.clone(), numbers: fizzbuzz_numbers.clone() }];
    for i in 1i64..32i64 {
        let string: _ = fizzbuzzs[if weekends.contains(&i) { 1i64 } else { 0i64 } as usize].get_string(i);
        println!("{:?}", &format!("{}: {}", i, string));
    }
}