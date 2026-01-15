# v1_7_0_any_method_kwargs.py
# V1.7.0: Any method call with kwargs should use BridgeMethodCall + kwargs

import pandas as pd

def main() -> None:
    data: dict[str, list[int]] = {"a": [2, 1], "b": [4, 3]}
    df = pd.DataFrame(data)
    text: str = df.to_string(index=False)
    print(text)

if __name__ == "__main__":
    main()
