"""
Fibonacci Benchmark - Recursive Version
Used to measure function call overhead (Python vs Rust)
"""

def fib(n: int) -> int:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

def main():
    # N=35 takes several seconds in Python, milliseconds in Rust
    n: int = 35
    result: int = fib(n)
    print(f"fib({n}) = {result}")

main()
