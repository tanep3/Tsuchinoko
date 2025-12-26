# ir_base.py

class TsuchinokoNode:
    id_counter = 0
    scope_tree = {}  # node_id -> parent_scope_id

    def __init__(self, node, matcher=None, indent_level=0, scope_parent_id=None):
        self.node = node
        self.matcher = matcher
        self.indent_level = indent_level
        self.node_id = TsuchinokoNode.id_counter
        self.scope_parent_id = scope_parent_id
        TsuchinokoNode.scope_tree[self.node_id] = scope_parent_id
        TsuchinokoNode.id_counter += 1

    @classmethod
    def matches(cls, node):
        raise NotImplementedError

    # def emit_c(self):
    #     raise NotImplementedError

    def emit_rust(self):
        raise NotImplementedError

    def emit_expression(self):
        return self.emit_rust()

    def emit_statement(self):
        indent = "    " * self.indent_level
        return indent + self.emit_rust()

    def normalize(self):
        pass  # 必要に応じて各ノードでオーバーライド
