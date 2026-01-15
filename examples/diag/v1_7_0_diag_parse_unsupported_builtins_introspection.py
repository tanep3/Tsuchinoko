# Expect: parse-phase diagnostics (dir/vars unsupported)
class Box:
    def __init__(self, value):
        self.value = value

box = Box(1)
_ = dir(box)
_ = vars(box)
