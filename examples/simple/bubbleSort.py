# バブルソート - 型ヒント付きバージョン
# Tsuchinoko変換用サンプル

def bubbleSort(lists: list[int]) -> tuple[list[int], int]:
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
    return sorted_list, list_length

def program_start() -> None:
    test_list: list[int] = [64, 34, 25, 12, 22, 11, 90]
    print("元のリスト:", test_list)
    sorted_lists, length = bubbleSort(test_list)
    print("ソート後のリスト:", sorted_lists)

if __name__ == "__main__":
    program_start()
