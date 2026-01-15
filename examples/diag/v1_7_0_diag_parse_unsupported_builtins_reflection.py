# Expect: parse-phase diagnostics (getattr/setattr/hasattr/delattr unsupported)
class Box:
    def __init__(self, value):
        self.value = value

box = Box(1)
_ = getattr(box, "value")
setattr(box, "value", 2)
_ = hasattr(box, "value")
delattr(box, "value")
