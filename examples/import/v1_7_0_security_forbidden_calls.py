import builtins as b
from typing import Callable, Any


def assert_security_violation(fn: Callable[[], Any], label: str) -> None:
    try:
        fn()
        raise ValueError(f"Expected SecurityViolation for {label}")
    except Exception as e:
        msg = str(e)
        if "Security" not in msg and "SecurityViolation" not in msg:
            raise


def main() -> None:
    assert_security_violation(lambda: b.eval("1+1"), "eval")
    assert_security_violation(lambda: b.exec("x = 1"), "exec")
    assert_security_violation(lambda: b.globals(), "globals")
    assert_security_violation(lambda: b.locals(), "locals")

    print("Security forbidden calls test passed")


if __name__ == "__main__":
    main()
