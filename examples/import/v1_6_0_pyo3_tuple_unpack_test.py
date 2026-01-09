# V1.6.0 FT-008: PyO3 タプルアンパッキングテスト
# カメラの read() のような1つの呼び出しがタプルを返し、複数変数に展開するケース

import cv2

def test_camera_read() -> None:
    # VideoCapture.read() はタプル (bool, frame) を返す
    cap = cv2.VideoCapture(0)
    ret, frame = cap.read()  # ← これが FT-008 のターゲット
    
    if ret:
        print("Frame captured successfully")
    else:
        print("Failed to capture frame")

def program_start() -> None:
    test_camera_read()

if __name__ == "__main__":
    program_start()
