import pandas as pd
def main() -> None:
    print("v1.7.0 Iterator Batch System Test")

    # Create data from a predictable range so we can verify the sum.
    count: int = 1_234
    df: pd.DataFrame = pd.DataFrame({"value": list(range(count))})

    print("Checking iteration/aggregation over Bridge-backed Series")
    total: int = 0
    for item in df["value"]:
        total += int(item)
    expected: int = (count - 1) * count // 2

    print("Computed total:", total)
    print("Expected total:", expected)

    if total != expected:
        raise ValueError("Iterator batch verification failed")

    print("Iterator batch verification passed")


if __name__ == "__main__":
    main()
