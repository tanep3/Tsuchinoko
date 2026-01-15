# Expect: semantic-phase diagnostics (custom context manager unsupported)
def ctx():
    return 1

with ctx():
    print("x")
