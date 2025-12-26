# Optional Type Example - Testing Optional[T] -> Option<T>

def find_item(items: list[int], target: int) -> Optional[int]:
    for i in range(len(items)):
        if items[i] == target:
            return i
    return None

def main() -> None:
    numbers: list[int] = [10, 20, 30, 40, 50]
    result: Optional[int] = find_item(numbers, 30)
    print("Found at index:", result)
    
    not_found: Optional[int] = find_item(numbers, 99)
    print("Not found:", not_found)

if __name__ == "__main__":
    main()
