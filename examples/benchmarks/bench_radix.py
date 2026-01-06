# Radix Sort Benchmark
# N = 10000000

def lcg(seed: int) -> int:
    return (1664525 * seed + 1013904223) % 4294967296

def generate_random_list(n: int) -> list[int]:
    l: list[int] = []
    seed: int = 12345
    for i in range(n):
        seed = lcg(seed)
        l.append(seed % 10000)
    return l

def getOrder(num: int) -> int:
    digits: int = len(str(num))
    return 10 ** (digits - 1)

def sortInOrder(lists: list[int], order: int) -> list[int]:
    if order == 0:
        return lists
    # Nested list comprehension is not supported yet, unroll it
    number_list: list[list[int]] = []
    k: int = 0
    for k in range(10):
        # Empty list literal with type hint logic... 
        # Tsuchinoko might need explicit empty list assignment or just []
        empty: list[int] = [] 
        number_list.append(empty)
        
    n: int = 0
    for n in lists:
        idx: int = (n % (order * 10)) // order
        number_list[idx].append(n)
        
    sorted_list: list[int] = []
    i: int = 0
    for i in range(10):
        sorted_list.extend(sortInOrder(number_list[i], order // 10))
    return sorted_list

def recursiveRadixSort(lists: list[int]) -> list[int]:
    if len(lists) == 0:
        return lists
    max_value: int = max(lists)
    sorted_list: list[int] = sortInOrder(lists, getOrder(max_value))
    return sorted_list

def main() -> None:
    print("Generating data (N=10000000)...")
    data: list[int] = generate_random_list(10000000)
    
    print("Sorting...")
    sorted_data: list[int] = recursiveRadixSort(data)
    
    print("Done.")
    print(sorted_data[0])
    print(sorted_data[1])
    print(sorted_data[2])

if __name__ == "__main__":
    main()
