# v1_4_from_import_test.py - from import syntax test
# V1.4.0: Test for "from module import name" support
#
# Tests that:
# 1. from numpy import mean is recognized
# 2. mean([1, 2, 3]) is converted to py_bridge.call_json("numpy.mean", ...)
# 3. --project is required

from numpy import mean

def calc_mean(values: list[float]) -> float:
    return mean(values)

def main() -> None:
    print("=== From Import Test ===")
    
    values: list[float] = [1.0, 2.0, 3.0, 4.0, 5.0]
    result: float = calc_mean(values)
    print(f"Mean of {values} = {result}")
    
    print("=== Done ===")

if __name__ == "__main__":
    main()
