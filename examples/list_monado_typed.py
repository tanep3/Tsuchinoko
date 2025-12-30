# list_monad_typed.py - 厳密型付き版 List Monad
# Tsuchinoko V1.2.0 テスト用

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Optional, Tuple


Action = Callable[[], Tuple[int, "ListMonad"]]


@dataclass
class ListMonad:
    """遅延評価リスト（モナド風）: int 専用"""
    _action: Optional[Action] = None
    _empty: bool = True
    _cached: bool = False
    _cache: Optional[Tuple[int, "ListMonad"]] = None

    def run(self) -> Tuple[int, "ListMonad"]:
        """cons-cell を (head, tail) で返す"""
        if self._empty:
            raise ValueError("Empty list has no (head, tail).")

        if not self._cached:
            if self._action is None:
                # empty 以外で action が None は想定外
                raise ValueError("Non-empty list must have an action.")
            self._cache = self._action()
            self._cached = True

        if self._cache is None:
            raise ValueError("Cache is None.")
        return self._cache

    def is_empty(self) -> bool:
        return self._empty

    @staticmethod
    def empty() -> "ListMonad":
        return ListMonad(None, True)

    def cons(self, value: int) -> "ListMonad":
        return ListMonad(lambda: (value, self), False)

    def head(self) -> int:
        h, _ = self.run()
        return h

    def tail(self) -> "ListMonad":
        _, t = self.run()
        return t

    def length(self) -> int:
        n: int = 0
        cur: ListMonad = self
        while not cur.is_empty():
            n += 1
            cur = cur.tail()
        return n


def test_list_monad() -> None:
    print("=== List Monad Test ===")

    empty: ListMonad = ListMonad.empty()
    print(f"empty.is_empty() = {empty.is_empty()}")

    one: ListMonad = empty.cons(1)
    two: ListMonad = one.cons(2)
    three: ListMonad = two.cons(3)

    print(f"three.head() = {three.head()}")
    print(f"three.length() = {three.length()}")

    print("=== Done ===")


def main() -> None:
    test_list_monad()


if __name__ == "__main__":
    main()
