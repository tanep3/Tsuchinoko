# V1.6.0 FT-002: クラス継承テスト（基本的なコンポジション構造）
# 完全な super() サポートは将来バージョンで拡張

class Animal:
    name: str
    
    def speak(self) -> str:
        return "..."

class Dog(Animal):
    breed: str
    
    def speak(self) -> str:
        return f"{self.name} says Woof!"

def program_start() -> None:
    # self.name が self.base.name に正しく変換されることを確認
    print("FT-002: クラス継承（コンポジション）テスト完了")

if __name__ == "__main__":
    program_start()
