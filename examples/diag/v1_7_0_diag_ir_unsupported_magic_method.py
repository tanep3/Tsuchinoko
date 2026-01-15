# Expect: lowering-phase diagnostics (unsupported magic method)
class MyIter:
    def __iter__(self):
        return self

def main() -> None:
    _ = MyIter()

if __name__ == "__main__":
    main()
