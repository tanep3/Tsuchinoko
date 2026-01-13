# v1_7_0_any_attr_item_slice.py
# V1.7.0: Any attribute/item/slice access should use bridge

import numpy as np

def main() -> None:
    arr = np.array([1, 2, 3, 4])
    first = arr[0]
    mid = arr[1:3]
    shape = arr.shape
    print(first)
    print(mid)
    print(shape)

if __name__ == "__main__":
    main()
