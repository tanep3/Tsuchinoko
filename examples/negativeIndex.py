# Negative Index Example - Testing arr[-1] support

def get_last_element(arr: list[int]) -> int:
    return arr[-1]

def get_second_last(arr: list[int]) -> int:
    return arr[-2]

def main() -> None:
    numbers: list[int] = [10, 20, 30, 40, 50]
    print("Last element:", get_last_element(numbers))
    print("Second last:", get_second_last(numbers))

if __name__ == "__main__":
    main()
