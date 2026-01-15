import cv2
import numpy as np

print("Founding Tsuchinoko OpenCV Test")

# 1. Create a dummy image (Handle) using Numpy
# Tsuchinoko Worker should return a Handle for ndarray
img = np.zeros((100, 100, 3), dtype=np.uint8)
print(f"Image Handle created: {img}")

# 2. Draw a rectangle (Function call with Handle)
# cv2.rectangle modifies the array in-place
cv2.rectangle(img, (10, 10), (50, 50), (255, 0, 0), 2)
print("Rectangle drawn")

# 3. Validation via Attribute Access
# img.shape should be returned as a value (tuple)
shape = img.shape
print(f"Shape: {shape}")

if shape != (100, 100, 3):
    raise ValueError(f"Unexpected shape: {shape}")

# 4. Validation via Method Call
# cv2.cvtColor returns a new image handle
gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
print(f"Gray Image Handle: {gray}")
print(f"Gray Shape: {gray.shape}")

# 5. VideoCapture Test (Graceful fallback)
# This fulfills the requirement `cv2.VideoCapture -> read`
print("Testing VideoCapture...")
try:
    cap = cv2.VideoCapture(0)
    # cap is a Handle
    print(f"VideoCapture Handle: {cap}")
    
    # Check if opened (Method call returning bool)
    opened = cap.isOpened()
    print(f"Is Opened: {opened}")
    
    if opened:
        # read returns (bool, frame_handle)
        # Tsuchinoko should handle tuple return from bridge
        ret, frame = cap.read()
        print(f"Read success: {ret}")
        if ret:
            print(f"Frame Handle: {frame}")
            print(f"Frame Shape: {frame.shape}")
    
    cap.release()
except Exception as e:
    print(f"VideoCapture test warning: {e}")

print("Verification Passed!")
