# test_for_hoisting.py
# Variable defined inside for loop, used outside

def find_first_even(nums: list) -> int:
    for n in nums:
        if n % 2 == 0:
            found: int = n
            break
    else:
        found: int = -1
    
    # 'found' should be accessible here (Python semantics)
    return found

def sum_loop(limit: int) -> int:
    total: int = 0
    for i in range(limit):
        total = total + i
    # 'i' should be accessible after loop (Python semantics)
    return i

def main() -> None:
    print(find_first_even([1, 3, 4, 5, 6]))  # Expected: 4
    print(find_first_even([1, 3, 5, 7]))     # Expected: -1
    print(sum_loop(5))                        # Expected: 4 (last value of i)

if __name__ == "__main__":
    main()
