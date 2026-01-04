# 単純なクラス例 - Phase E テスト用
class Counter:
    def __init__(self, start: int):
        self.__count: int = start
    
    def increment(self) -> int:
        self.__count = self.__count + 1
        return self.__count

if __name__ == "__main__":
    c: Counter = Counter(0)
    print(c.increment())
