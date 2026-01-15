# Test for unsupported builtins: memoryview() and bytearray()
# This should trigger TNK-UNSUPPORTED-SYNTAX diagnostic for both

data = b"hello"
mv = memoryview(data)
ba = bytearray(5)
print(mv, ba)
