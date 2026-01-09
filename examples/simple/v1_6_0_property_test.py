# V1.6.0 FT-003: @property デコレーターテスト

class Circle:
    def __init__(self, radius: float) -> None:
        self._radius = radius
    
    @property
    def radius(self) -> float:
        return self._radius
    
    @radius.setter
    def radius(self, value: float) -> None:
        if value < 0:
            raise ValueError("Radius must be positive")
        self._radius = value
    
    def area(self) -> float:
        return 3.14159 * self._radius * self._radius

def program_start() -> None:
    circle: Circle = Circle(5.0)
    
    # Test 1: Property getter
    r: float = circle.radius()  # Note: Rust では関数呼び出し形式になる
    print(f"Initial radius: {r}")
    
    # Test 2: Area calculation
    a: float = circle.area()
    print(f"Area: {a}")

if __name__ == "__main__":
    program_start()
