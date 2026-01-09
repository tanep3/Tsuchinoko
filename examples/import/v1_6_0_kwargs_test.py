# V1.6.0 FT-006: **kwargs テスト
# NOTE: 現バージョンでは kwargs がある関数の定義のみサポート
#       呼び出し側で空の HashMap を自動生成する機能は未対応

def greet(name: str, **kwargs) -> str:
    greeting: str = kwargs.get("greeting", "Hello")
    return f"{greeting}, {name}!"

def program_start() -> None:
    # kwargs なし呼び出しは現バージョンでは未サポート
    # 将来的に空の HashMap を自動生成する予定
    print("FT-006: kwargs 定義テスト完了")

if __name__ == "__main__":
    program_start()
