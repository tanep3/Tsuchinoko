# v1_2_simple_test.py - Tsuchinoko V1.2.0 simple tests

def sum_all(*values: int) -> int:
    # 可変長引数を受け取って合計を返す
    total: int = 0
    for v in values:
        total += v
    return total


def test_varargs() -> None:
    result: int = sum_all(1, 2, 3, 4, 5)
    print(f"sum_all(1,2,3,4,5) = {result}")


def main() -> None:
    print("=== V1.2.0 Simple Test ===")
    test_varargs()
    print("=== Done ===")


if __name__ == "__main__":
    main()
