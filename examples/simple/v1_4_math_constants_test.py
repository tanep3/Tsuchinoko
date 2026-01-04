# v1_4_math_constants_test.py - math constants test
# V1.4.0: Test for math.pi, math.e, math.tau, math.inf, math.nan

import math

def main() -> None:
    print("=== Math Constants Test ===")
    
    # Constants
    pi: float = math.pi
    e: float = math.e
    tau: float = math.tau
    
    print(f"math.pi = {pi}")
    print(f"math.e = {e}")
    print(f"math.tau = {tau}")
    
    # Basic calculations using constants
    circle_area: float = pi * 2.0 * 2.0  # Area of circle with radius 2
    print(f"Circle area (r=2) = {circle_area}")
    
    print("=== Done ===")

if __name__ == "__main__":
    main()
