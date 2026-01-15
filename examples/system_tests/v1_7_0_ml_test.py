
# Machine Learning Integration Test
# libraries: scikit-learn, numpy

import numpy as np
from sklearn.linear_model import LinearRegression
from sklearn.datasets import make_regression

def test_ml_pipeline():
    print("Testing ML Pipeline...")
    
    # 1. Generate Data (Handle returns tuple of arrays)
    # make_regression returns (X, y)
    print("1. Generating Data...")
    X, y = make_regression(n_samples=100, n_features=2, noise=0.1)
    
    # Check shapes (Attribute access on ndarray handle)
    print(f"X shape: {X.shape}")
    print(f"y shape: {y.shape}")
    
    # 2. Initialize Model
    print("2. Initializing Model...")
    model = LinearRegression()
    # model is a Handle to LinearRegression instance
    
    # 3. Train Model
    print("3. Training Model...")
    model.fit(X, y)
    
    # Check attributes (coeffs)
    print(f"Coefficients: {model.coef_}")
    print(f"Intercept: {model.intercept_}")
    
    # 4. Predict
    print("4. Predicting...")
    predictions = model.predict(X)
    
    # Verify
    print(f"Predictions shape: {predictions.shape}")
    
    if predictions.shape == y.shape:
        print("Verification Passed: Prediction shape matches.")
    else:
        raise Exception(f"Verification Failed: Shape mismatch {predictions.shape} != {y.shape}")

if __name__ == "__main__":
    test_ml_pipeline()
