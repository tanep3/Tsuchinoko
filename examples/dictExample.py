# Simple Dict Example - Basic dict support test
# dict[str, int] -> HashMap<String, i64>

def main() -> None:
    scores: dict[str, int] = {"alice": 100, "bob": 85}
    print("Scores:", scores)

if __name__ == "__main__":
    main()
