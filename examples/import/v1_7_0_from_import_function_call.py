# v1_7_0_from_import_function_call.py
# V1.7.0: from-import function call should use bridge and propagate py_bridge

from numpy import mean

def calc_mean(values: list[float]) -> float:
    return mean(values)

def main() -> None:
    values: list[float] = [1.0, 2.0, 3.0]
    result: float = calc_mean(values)
    print(result)

if __name__ == "__main__":
    main()
