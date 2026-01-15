# Expect: parse-phase diagnostics (id/hash unsupported)
class Box:
    def __init__(self, value):
        self.value = value

box = Box(1)
_ = id(box)
_ = hash(box)
