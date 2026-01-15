# Test for unsupported builtin: compile()
# This should trigger TNK-UNSUPPORTED-SYNTAX diagnostic

code = compile("1 + 1", "<string>", "eval")
result = eval(code)
print(result)
