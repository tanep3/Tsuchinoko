# v1_7_0_json_conversion_result.py
# V1.7.0: JsonConversion should require TsuchinokoError even without bridge

def main() -> None:
    val: object = 5
    i: int = val
    f: float = val
    s: str = "ok"
    print(i)
    print(f)
    print(s)

if __name__ == "__main__":
    main()
