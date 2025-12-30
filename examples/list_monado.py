# NOTE:
# This is NOT a practical list implementation.
# This example demonstrates Tsuchinoko-style evaluation,
# immutability, and monadic composition.

class List:
    def __init__(self, action=None, empty=True):
        # empty=True のとき action は不要
        self._action = action
        self._empty = empty

        # thunk の多重評価を避ける（遅延としての説得力）
        self._cached = False
        self._cache = None  # (head, tail)

    # ---- core ----
    def run(self):
        """cons-cell を (head, tail) で返す。empty は例外。"""
        if self._empty:
            raise ValueError("Empty list has no (head, tail).")
        if not self._cached:
            self._cache = self._action()
            self._cached = True
        return self._cache

    def isEmpty(self):
        return self._empty

    @staticmethod
    def empty():
        return List(empty=True)

    @staticmethod
    def init(*values):
        if not values:
            return List.empty()
        head, *tail = values
        return List.empty().cons(head).append(List.init(*tail))

    # construct: 先頭に追加
    def cons(self, value):
        return List(lambda: (value, self), empty=False)

    def head(self):
        head, _ = self.run()
        return head

    def tail(self):
        _, tail = self.run()
        return tail

    # ---- list ops ----
    def append(self, other):
        """連結 (this ++ other)"""
        if self.isEmpty():
            return other
        # laziness を保つ：head/tail は thunk 内で評価される
        return List(lambda: (self.head(), self.tail().append(other)), empty=False)

    def snoc(self, value):
        """末尾に1要素追加"""
        return self.append(List.empty().cons(value))

    def length(self, limit=None):
        """長さ（必要なら limit で打ち切り可能）"""
        n = 0
        cur = self
        while not cur.isEmpty():
            n += 1
            if limit is not None and n >= limit:
                break
            cur = cur.tail()
        return n

    def pop(self):
        """先頭を削除して (tail, head) を返す"""
        if self.isEmpty():
            return self, None
        return self.tail(), self.head()

    def popTail(self):
        """末尾を削除して (new_list, last) を返す（学習用：O(n)）"""
        if self.isEmpty():
            return self, None
        if self.tail().isEmpty():
            return List.empty(), self.head()
        new_tail, last = self.tail().popTail()
        return List(lambda: (self.head(), new_tail), empty=False), last

    # ---- monad-ish ----
    def map(self, func):
        if self.isEmpty():
            return List.empty()
        return List(lambda: (func(self.head()), self.tail().map(func)), empty=False)

    @staticmethod
    def concat(list_of_lists):
        """flatten: List[List[a]] -> List[a]"""
        if list_of_lists.isEmpty():
            return List.empty()
        return list_of_lists.head().append(List.concat(list_of_lists.tail()))

    def bind(self, func):
        """
        List monad bind:
          xs >>= f = concat(map f xs)
        """
        return List.concat(self.map(func))

    # ---- debug / display ----
    def to_pylist(self, limit=None):
        out = []
        cur = self
        while not cur.isEmpty():
            out.append(cur.head())
            if limit is not None and len(out) >= limit:
                break
            cur = cur.tail()
        return out


# -------------------------
# テスト
# -------------------------
a = List.init()
b = a.snoc("たね")
c = b.snoc("タルモン")

print(f"長さ: {c.length()}")
print(c.to_pylist())

d = List.init(1, 2, 3, 4)
print(d.to_pylist())
print(f"長さ: {d.length()}")  # → 4
e, last = d.popTail()
print(f"末尾削除: {last}")    # → 4
print(f"長さ: {e.length()}")  # → 3
f, first = e.pop()
print(f"先頭削除: {first}")   # → 1
print(f"長さ: {f.length()}")  # → 2
print(f.to_pylist())

# モナドっぽさの確認（非決定性の合成）
xs = List.init(1, 2)
ys = List.init(10, 20)
pairs_sum = xs.bind(lambda x: ys.map(lambda y: x + y))
print(pairs_sum.to_pylist())   # [11, 21, 12, 22]
