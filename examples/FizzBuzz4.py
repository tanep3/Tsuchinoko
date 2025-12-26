from typing import Callable, Dict, Tuple, List

# 条件関数型のインターフェース
ConditionFunction = Callable[[int, int], bool]

# 条件クラス
class Condition:
    def __init__(self, condition_function: ConditionFunction):
        self.__condition_function = condition_function

    def check(self, num: int, key_num: int) -> bool:
        return self.__condition_function(num, key_num)

# 数字クラス
class Numbers:
    def __init__(self, numbers: Dict[int, str] = {}):
        self.__numbers = dict(sorted(numbers.items()))

    def add(self, key_num: int, name: str) -> 'Numbers':
        if key_num in self.__numbers:
            raise ValueError('数値が重複しています。')
        new_numbers = dict(self.__numbers)
        new_numbers[key_num] = name
        return Numbers(new_numbers)

    def items(self) -> List[Tuple[int, str]]:
        return list(self.__numbers.items())

# FizzBuzzのビジネスロジッククラス
class FizzBuzz:
    def __init__(self, condition: Condition, numbers: Numbers):
        self.__condition = condition
        self.__numbers = numbers

    def get_string(self, num: int) -> str:
        return ''.join(name for key_num, name in self.__numbers.items() if self.__condition.check(num, key_num) )

# 条件関数
def weekday_condition(num: int, key_num: int) -> bool:
    return num % key_num == 0

def weekend_condition(num: int, key_num: int) -> bool:
    return num % 2 == 1 and num % key_num == 0

# 週末の日付
weekends = tuple(d for d in range(1, 32) if (d-1) % 7 == 0 or d % 7 == 0)

# FizzBuzzの数値
fizzbuzz_numbers = Numbers({3: 'Fizz', 5: 'Buzz', 7: 'Lazz', 11: 'Pozz'})

# FizzBuzzの実行
fizzbuzzs = [
    FizzBuzz(Condition(weekday_condition), fizzbuzz_numbers),
    FizzBuzz(Condition(weekend_condition), fizzbuzz_numbers)
]

for i in range(1, 32):
    string = fizzbuzzs[1 if i in weekends else 0].get_string(i)
    print(f'{i}: {string}')
