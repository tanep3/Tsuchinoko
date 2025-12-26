# 再帰的基数ソート - 型ヒント付きバージョン
# Tsuchinoko変換用サンプル

def getOrder(num: int) -> int:
    digits: int = len(str(num))
    return 10 ** (digits - 1)

def sortInOrder(lists: list[int], order: int) -> list[int]:
    if order == 0:
        return lists
    number_list: list[list[int]] = [[] for value in range(10)]
    n: int = 0
    for n in lists:
        # そのオーダーの数値でリストに入れる
        idx: int = (n % (order * 10)) // order
        number_list[idx].append(n)
    sorted_list: list[int] = []
    i: int = 0
    for i in range(10):
        sorted_list.extend(sortInOrder(number_list[i], order // 10))
    return sorted_list

def recursiveRadixSort(lists: list[int]) -> list[int]:
    max_value: int = max(lists)
    sorted_list: list[int] = sortInOrder(lists, getOrder(max_value))
    return sorted_list

def main() -> None:
    test_list: list[int] = [170, 45, 75, 90, 802, 24, 2, 66]
    print("元のリスト:", test_list)
    result: list[int] = recursiveRadixSort(test_list)
    print("ソート後:", result)

if __name__ == "__main__":
    main()
