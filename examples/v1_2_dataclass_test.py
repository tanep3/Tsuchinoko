# v1_2_dataclass_test.py - @dataclass テスト

from dataclasses import dataclass


@dataclass
class Point:
    x: int
    y: int


@dataclass
class Person:
    name: str
    age: int


def main() -> None:
    print("=== @dataclass Test ===")
    
    p1: Point = Point(10, 20)
    print(f"Point: x={p1.x}, y={p1.y}")
    
    p2: Person = Person("Tane", 30)
    print(f"Person: {p2.name}, age={p2.age}")
    
    print("=== Done ===")


if __name__ == "__main__":
    main()
