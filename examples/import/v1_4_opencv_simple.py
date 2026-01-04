# v1_4_opencv_simple.py - OpenCV simple test (headless)
# V1.4.0: Test for automatic external module detection
#
# Tests that:
# 1. import cv2 is recognized as external module
# 2. cv2.getVersionString() is converted to py_bridge.call_json()
# 3. --project is required (not just -o)

import cv2

def get_opencv_build_info() -> str:
    return cv2.getBuildInformation()

def main() -> None:
    print("=== OpenCV Test ===")
    
    # Get OpenCV build info (function call, not attribute access)
    info: str = get_opencv_build_info()
    # Just check if it's a non-empty string
    if len(info) > 0:
        print("OpenCV build info retrieved successfully")
        print(f"Info length: {len(info)} chars")
    
    print("=== Done ===")

if __name__ == "__main__":
    main()
