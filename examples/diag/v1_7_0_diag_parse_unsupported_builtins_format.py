# Expect: parse-phase diagnostics (format/repr unsupported)
class Box:
    def __init__(self, value):
        self.value = value

box = Box(1)
_ = format(123, "d")
_ = repr(box)
