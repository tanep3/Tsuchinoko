# PyO3 サポート設計

## 概要
- `import numpy as np` や `import pandas as pd` を検出
- 純 Rust ではなく PyO3 ブリッジコードを生成
- Python ランタイムとの連携

## 実装アプローチ

### Phase 1: import 検出
- パーサーで `import X as Y` / `from X import Y` を認識
- `SemanticAnalyzer` に `pyo3_imports: Vec<(module, alias)>` を追加

### Phase 2: PyO3 プレリュード
numpy/pandas import がある場合、以下を自動追加:
```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;
```

### Phase 3: main ラッパー
```rust
fn main() -> PyResult<()> {
    Python::with_gil(|py| {
        // venv 連携
        if let Ok(venv) = std::env::var("TSUCHINOKO_VENV") {
            let sys = py.import("sys")?;
            sys.getattr("path")?
                .call_method1("insert", (0, format!("{}/lib/python3.11/site-packages", venv)))?;
        }
        
        // ユーザーコード
        _user_main(py)?;
        Ok(())
    })
}

fn _user_main(py: Python<'_>) -> PyResult<()> {
    // 元の main の内容
}
```

### Phase 4: np/pd 呼び出し変換
- `np.array([1, 2, 3])` → `py.import("numpy")?.call_method1("array", ([1, 2, 3],))?`
- 戻り値は `PyAny` 型

## Cargo.toml 要件
```toml
[dependencies]
pyo3 = { version = "0.20", features = ["auto-initialize"] }
```

## 環境変数
- `TSUCHINOKO_VENV`: Python venv パス（設定されていれば site-packages を追加）
