# V1.6.0 FT-002: super() テスト

class Animal:
    name: str
    def speak(self) -> str:
        return "..."

class Dog(Animal):
    breed: str
    def speak(self) -> str:
        parent = super().speak()
        return f"{parent} Woof!"

def program_start() -> None:
    dog: Dog = Dog("Rex", "Labrador")
    print(dog.speak())

if __name__ == "__main__":
    program_start()
