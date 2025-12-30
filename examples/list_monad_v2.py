# list_monad_v2.py - Tsuchinoko V1.2.0 テスト用
# 型エイリアスを使わないシンプルなバージョン

from dataclasses import dataclass
from typing import Optional


@dataclass
class IntList:
    """遅延評価シンプルリスト: int 専用"""
    _head: int = 0
    _tail: Optional["IntList"] = None
    _empty: bool = True

    def is_empty(self) -> bool:
        return self._empty

    @staticmethod
    def empty() -> "IntList":
        return IntList(0, None, True)

    def cons(self, value: int) -> "IntList":
        return IntList(value, self, False)

    def head(self) -> int:
        if self._empty:
            raise ValueError("Empty list has no head")
        return self._head

    def tail(self) -> "IntList":
        if self._empty:
            raise ValueError("Empty list has no tail")
        if self._tail is None:
            return IntList.empty()
        return self._tail

    def length(self) -> int:
        n: int = 0
        cur: IntList = self
        while not cur.is_empty():
            n += 1
            cur = cur.tail()
        return n


def test_list() -> None:
    print("=== IntList Test ===")
    
    empty: IntList = IntList.empty()
    print(f"empty.is_empty() = {empty.is_empty()}")
    
    one: IntList = empty.cons(1)
    two: IntList = one.cons(2)
    three: IntList = two.cons(3)
    
    print(f"three.head() = {three.head()}")
    print(f"three.length() = {three.length()}")
    
    print("=== Done ===")


def main() -> None:
    test_list()


if __name__ == "__main__":
    main()
