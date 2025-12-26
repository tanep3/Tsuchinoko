# compiler.py

import ast
from src.matcher import TsuchinokoMatcher

matcher = TsuchinokoMatcher()

def compile_python_to_rust(source_code: str) -> str:
    tree = ast.parse(source_code)
    root = matcher.match_node(tree)
    if root is None:
        raise RuntimeError("Moduleルートが処理できません")
    return emit_recursive(root)

def emit_recursive(node_obj):
    node_obj.normalize()
    return node_obj.emit_rust()
