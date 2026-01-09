# V1.6.0 FT-005: DynamicValue enum 自動生成テスト

def process(value):
    if isinstance(value, str):
        return "is string"
    elif isinstance(value, int):
        return "is int"
    return "unknown"

def program_start() -> None:
    print("FT-005: DynamicValue enum テスト完了")

if __name__ == "__main__":
    program_start()
