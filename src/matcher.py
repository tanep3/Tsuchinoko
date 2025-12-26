# matcher.py

from src.ir_nodes import (
    TsuchinokoModule, TsuchinokoFunctionDef, TsuchinokoAssign, TsuchinokoExpr,
    TsuchinokoCall, TsuchinokoFor, TsuchinokoIf, TsuchinokoReturn,
    TsuchinokoCompare, TsuchinokoBinOp, TsuchinokoName, TsuchinokoConstant,
    TsuchinokoTuple, TsuchinokoList, TsuchinokoSubscript, TsuchinokoAttribute,
    TsuchinokoPrint, TsuchinokoCallList, TsuchinokoCallLen, TsuchinokoContinue,
    TsuchinokoCallRange
)

NODE_CLASSES = [
    TsuchinokoModule,
    TsuchinokoFunctionDef,
    TsuchinokoAssign,
    TsuchinokoExpr,
    TsuchinokoCall,
    TsuchinokoFor,
    TsuchinokoIf,
    TsuchinokoReturn,
    TsuchinokoCompare,
    TsuchinokoBinOp,
    TsuchinokoName,
    TsuchinokoConstant,
    TsuchinokoTuple,
    TsuchinokoList,
    TsuchinokoSubscript,
    TsuchinokoAttribute,
    TsuchinokoPrint,
    TsuchinokoCallList,
    TsuchinokoCallLen,
    TsuchinokoContinue,
    TsuchinokoCallRange
]

class TsuchinokoMatcher:
    def match_node(self, node, indent_level=0, scope_parent_id=None):
        for cls in NODE_CLASSES:
            if cls.matches(node):
                return cls(node, self, indent_level, scope_parent_id)
        if node is None:
            print("[WARN] 未対応ノード: None")
            return None
        if hasattr(node, 'func') and hasattr(node.func, 'id'):
            print(f"[WARN] 未対応ノード: {type(node).__name__} {node.func.id}")
            return None
        print(f"[WARN] 未対応ノード: {type(node).__name__}")
        return None
