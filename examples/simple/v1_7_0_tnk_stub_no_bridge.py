# v1_7_0_tnk_stub_no_bridge.py
# V1.7.0: TnkValue usage without bridge should emit standalone stub

def main() -> None:
    x: object = 123
    y: object = "hello"
    print(x)
    print(y)

if __name__ == "__main__":
    main()
