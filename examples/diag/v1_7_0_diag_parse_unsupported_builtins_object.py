# Test for unsupported builtin: object()
# This should trigger TNK-UNSUPPORTED-SYNTAX diagnostic

obj = object()
print(obj)
