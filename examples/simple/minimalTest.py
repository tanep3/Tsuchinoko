# 最小テスト - import なし、シンプルなCallable
def double(x: int) -> int:
    return x * 2

def main() -> None:
    y: int = double(5)
    print(y)
