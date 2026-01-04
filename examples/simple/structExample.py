from dataclasses import dataclass

@dataclass
class Point:
    x: int
    y: int

@dataclass
class Rectangle:
    width: int
    height: int

def distance_squared(p1: Point, p2: Point) -> int:
    dx: int = p1.x - p2.x
    dy: int = p1.y - p2.y
    return dx * dx + dy * dy

def area(rect: Rectangle) -> int:
    return rect.width * rect.height

def main() -> None:
    p1: Point = Point(0, 0)
    p2: Point = Point(3, 4)
    dist: int = distance_squared(p1, p2)
    print(dist)
    
    r: Rectangle = Rectangle(10, 5)
    a: int = area(r)
    print(a)

if __name__ == "__main__":
    main()
