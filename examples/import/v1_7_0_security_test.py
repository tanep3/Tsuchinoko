import builtins as b
import datetime as dt

def main():
    # 1) Security: forbidden eval
    try:
        b.eval("1+1")
        raise ValueError("Expected SecurityViolation for eval")
    except Exception as e:
        msg = str(e)
        if "Security" not in msg and "SecurityViolation" not in msg:
            raise

    # 2) Security: private attribute access
    obj = dt.datetime.now()
    try:
        _ = obj.__class__
        raise ValueError("Expected SecurityViolation for __class__")
    except Exception as e:
        msg = str(e)
        if "Security" not in msg and "SecurityViolation" not in msg:
            raise

    print("Security test passed")

if __name__ == "__main__":
    main()
