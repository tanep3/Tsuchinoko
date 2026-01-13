# v1_7_0_raise_in_method.py
# V1.7.0: Raise inside method should require TsuchinokoError in impl blocks

class Numbers:
    def set_value(self, value: int) -> int:
        if value < 0:
            raise ValueError("negative value")
        return value

def main() -> None:
    nums: Numbers = Numbers()
    print(nums.set_value(3))

if __name__ == "__main__":
    main()
