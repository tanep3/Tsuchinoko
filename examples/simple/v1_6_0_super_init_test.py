# V1.6.0 FT-002: super().__init__() テスト

class Animal:
    def __init__(self, name: str) -> None:
        self.name = name
    def speak(self) -> str:
        return "..."

class Dog(Animal):
    def __init__(self, name: str, breed: str) -> None:
        super().__init__(name)
        self.breed = breed
    def speak(self) -> str:
        return f"{self.name} says Woof!"

def program_start() -> None:
    dog: Dog = Dog("Rex", "Labrador")
    print(dog.speak())

if __name__ == "__main__":
    program_start()
