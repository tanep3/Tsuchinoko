# Expect: parse-phase diagnostics (type/issubclass unsupported)
class A:
    pass

class B(A):
    pass

_ = type(A())
_ = issubclass(B, A)
