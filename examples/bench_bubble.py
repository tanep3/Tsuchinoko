# Bubble Sort Benchmark
# N = 30000

def lcg(seed: int) -> int:
    # A simple LCG: x = (a * x + c) % m
    return (1664525 * seed + 1013904223) % 4294967296

def generate_random_list(n: int) -> list[int]:
    l: list[int] = []
    seed: int = 12345
    for i in range(n):
        seed = lcg(seed)
        l.append(seed % 10000)
    return l

def bubbleSort(lists: list[int]) -> list[int]:
    sorted_list: list[int] = list(lists)
    list_length: int = len(sorted_list)
    i: int = 0
    j: int = 0
    for i in range(list_length):
        for j in range(list_length - i - 1):
            if sorted_list[j] > sorted_list[j + 1]:
                temp: int = sorted_list[j]
                sorted_list[j] = sorted_list[j + 1]
                sorted_list[j + 1] = temp
    return sorted_list

def main() -> None:
    print("Generating data (N=30000)...")
    data: list[int] = generate_random_list(30000)
    
    print("Sorting...")
    # We won't measure time inside python script as we don't have time module
    # External 'time' command will be used
    sorted_data: list[int] = bubbleSort(data)
    
    # Verification (first few items)
    print("Done.")
    print(sorted_data[0])
    print(sorted_data[1])
    print(sorted_data[2])

if __name__ == "__main__":
    main()
