from typing import Callable, Dict

ConditionFunction = Callable[[int,int], bool]

#FizzBuzzのビジネスロジック
def FizzBuzz(func: ConditionFunction, divisionMessages:Dict[int,str]) -> Callable[[int], str]:
    def makeString(x:int) -> str:
        string = ''.join(message for keyNum, message in divisionMessages.items() if func(x,keyNum))
        return string
    return makeString

# 条件式の定義
def isDivisible(x:int, y:int) -> bool:
    return x % y == 0
def isDivisibleWeekEnd(x:int, y:int) -> bool:
    return x % 2 == 1 and x % y == 0

# FizzBuzzの数値
fizzbuzz_numbers: Dict[int,str] = {3: 'Fizz', 5: 'Buzz', 7: 'Lazz', 11: 'Pozz'}

# 週末の日付
weekends: tuple = tuple(d for d in range(1, 32) if (d-1) % 7 == 0 or d % 7 == 0)

for i in range(1,32):
    condition = isDivisibleWeekEnd if i in weekends else isDivisible
    string = FizzBuzz(condition, fizzbuzz_numbers)(i)
    print(f'{i}:{string}')
