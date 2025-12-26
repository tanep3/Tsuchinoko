# ir_nodes.py

import ast
from src.ir_base import TsuchinokoNode

INDENT = "    "

class TsuchinokoModule(TsuchinokoNode):
    """
    プログラム全体のIRルートノード。
    グローバルスコープの変数宣言と関数定義をまとめて保持。
    emit_rust() では全体のRustコードを構築し、関数宣言、main関数の構成を行う。
    ここで各トップレベルステートメント（AssignやFunctionDefなど）を分類・出力する。
    型推論などに基づく変数の型情報を適切にCコードに展開する責務を持つ。
    """
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Module)
    
    def emit_rust(self):
        functions = []
        setup_body = []

        for stmt in self.node.body:
            if isinstance(stmt, ast.FunctionDef):
                functions.append(self.matcher.match_node(stmt, scope_parent_id=self.node_id).emit_rust())
            elif isinstance(stmt, ast.If) and isinstance(stmt.test, ast.Compare):
                # if __name__ == '__main__': の処理
                if (isinstance(stmt.test.left, ast.Name) and stmt.test.left.id == '__name__' and
                    isinstance(stmt.test.ops[0], ast.Eq) and
                    isinstance(stmt.test.comparators[0], ast.Constant) and
                    stmt.test.comparators[0].value == '__main__'):
                    if len(setup_body) != 0:
                        main_lines = ["    setup();"]
                    else:
                        main_lines = []
                    main_lines += [self.matcher.match_node(s, indent_level=1, scope_parent_id=self.node_id).emit_statement() for s in stmt.body]
                    main_func = "fn main() {\n" + "\n".join(main_lines) + "\n}"
                    functions.append(main_func)
                    continue
                functions.append(self.matcher.match_node(stmt, indent_level=1, scope_parent_id=self.node_id).emit_statement())
            else:
                setup_body.append(self.matcher.match_node(stmt, indent_level=1, scope_parent_id=self.node_id).emit_statement())
            
            setup_func = ""
            if setup_body:
                setup_code = "\n".join(setup_body)
                setup_func = f"fn setup() {{\n{setup_code}\n}}"

        output = ""
        if setup_func:
            output = f"{setup_func}\n\n"
        if functions:
            output += "\n\n".join(functions)
        return output

class TsuchinokoFunctionDef(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.FunctionDef)

    def emit_rust(self):
        func_name = self.node.name
        args = [arg.arg for arg in self.node.args.args]
        arg_str = ", ".join([f"{arg}: i32" for arg in args])  # 仮に i32 としておく
        body_lines = [self.matcher.match_node(stmt, self.indent_level + 1, scope_parent_id=self.node_id).emit_statement() for stmt in self.node.body]
        body = "\n".join(body_lines)
        body = "\n".join([INDENT * self.indent_level + line for line in body_lines])
        return f"{self.node_id}fn {func_name}({arg_str}) {{\n{body}\n}}"

class TsuchinokoAssign(TsuchinokoNode):
    """
    Pythonの代入文（a = b）に対応。
    左辺と右辺の型に応じた代入文をCに変換。
    右辺がリスト系かどうかを見て、型推論を行う責任を持つ。
    代入対象がグローバルかローカルかにかかわらず、型を明示すべきかどうかは設計方針次第。
    Module では、Assignインスタンスに問い合わせて型を知るべき。
    """
    declared_vars = []

    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Assign)
    
    # def emit_rust(self):
    #     return self.emit_statment()
    
    def emit_rust(self):
        target_node = self.node.targets[0]
        value_node = self.node.value
        target = self.matcher.match_node(target_node, scope_parent_id=self.node_id).emit_expression()
        value = self.matcher.match_node(value_node, scope_parent_id=self.node_id).emit_expression()

        if isinstance(target_node, ast.Tuple):
            return f"({target}) = {value};"

        is_vec_init = isinstance(value_node, ast.List)
        if target not in TsuchinokoAssign.declared_vars:
            TsuchinokoAssign.declared_vars.append(target)
            if is_vec_init:
                return f"let mut {target} = {value};"
            return f"let mut {target} = {value};"
        else:
            return f"{target} = {value};"


    def emit_statment(self):
        target = self.matcher.match_node(self.node.targets[0], scope_parent_id=self.node_id).emit_expression()
        value = self.matcher.match_node(self.node.value, scope_parent_id=self.node_id).emit_expression()
        if target not in TsuchinokoAssign.declared_vars:
            TsuchinokoAssign.declared_vars.add(target)
            return f"let mut {target} = {value};"
        else:
            return f"{target} = {value};"

        # targets = [self.matcher.match_node(t).emit_expression() for t in self.node.targets]
        # value_node = self.node.value
        # value = self.matcher.match_node(value_node).emit_expression()
        # リスト定義なら let mut に変換
        # if TsuchinokoList.matches(value_node):
        #     return f"let mut {targets[0]} = {value};"
        # return f"{targets[0]} = {value};"

class TsuchinokoExpr(TsuchinokoNode):
    # 式ステートメント
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Expr)

    def emit_rust(self):
        return self.matcher.match_node(self.node.value, scope_parent_id=self.node_id).emit_expression() + ";"

class TsuchinokoCall(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        exclusions = (
            'print', 
            'list', 
            'len',
            'range'
        )
        if not isinstance(node, ast.Call):
            return False
        func_name = getattr(node.func, 'id', None)
        if func_name in exclusions:
            return False
        return True

    def emit_rust(self):
        func = self.matcher.match_node(self.node.func, scope_parent_id=self.node_id).emit_expression()
        args = [self.matcher.match_node(arg, scope_parent_id=self.node_id).emit_expression() for arg in self.node.args]
        return f"{func}({', '.join(args)})"


class TsuchinokoReturn(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Return)

    def emit_rust(self):
        if self.node.value:
            value = self.matcher.match_node(self.node.value, scope_parent_id=self.node_id).emit_expression()
            return f"return {value};"
        return "return;"

class TsuchinokoName(TsuchinokoNode):
    # 変数名
    # 定義済みかどうかと、型の情報を管理する。
    var_table = {}  # name -> {"declared": True, "type": str}

    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Name)

    def emit_rust(self):
        return str(self.node_id) + self.node.id

    @classmethod
    def where_declared(cls, name, current_scope_id):
        if name not in cls.var_table:
            return None

        candidates = cls.var_table[name]

        # スコープの親情報が使えるように、事前に TsuchinokoNode の全ノードを登録しておく
        from_scope = current_scope_id
        while from_scope is not None:
            for var in candidates:
                if var["scope_id"] == from_scope:
                    return var
            # スコープをたどっていく
            from_scope = TsuchinokoNode.scope_tree.get(from_scope)
        return None

    @classmethod
    def register(cls, name, var_type="i32"):
        if name not in cls.var_table:
            cls.var_table[name] = {"scope_id": True, "type": var_type}

    @classmethod
    def is_declared(cls, name):
        return name in cls.var_table
    

    

class TsuchinokoConstant(TsuchinokoNode):
    # リテラル定数
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Constant)

    def emit_rust(self):
        value = self.node.value
        if isinstance(value, str):
            return f'"{value}"'
        return str(value)

class TsuchinokoBinOp(TsuchinokoNode):
    # 二項演算
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.BinOp)

    def emit_rust(self):
        left = self.matcher.match_node(self.node.left, scope_parent_id=self.node_id).emit_expression()
        right = self.matcher.match_node(self.node.right, scope_parent_id=self.node_id).emit_expression()
        op = self.emit_operator(self.node.op)
        return f"({left} {op} {right})"

    def emit_operator(self, op):
        return {
            ast.Add: "+",
            ast.Sub: "-",
            ast.Mult: "*",
            ast.Div: "/",
            ast.Mod: "%",
        }.get(type(op), "?")

class TsuchinokoCompare(TsuchinokoNode):
    # 比較
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Compare)

    def emit_rust(self):
        left = self.matcher.match_node(self.node.left, scope_parent_id=self.node_id).emit_expression()
        comparisons = []
        for op, comparator in zip(self.node.ops, self.node.comparators):
            right = self.matcher.match_node(comparator, scope_parent_id=self.node_id).emit_expression()
            op_str = self.emit_operator(op)
            comparisons.append(f"{left} {op_str} {right}")
        return " && ".join(comparisons)

    def emit_operator(self, op):
        return {
            ast.Eq: "==",
            ast.NotEq: "!=",
            ast.Lt: "<",
            ast.LtE: "<=",
            ast.Gt: ">",
            ast.GtE: ">=",
        }.get(type(op), "?")

class TsuchinokoIf(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.If)

    def emit_rust(self):
        indent = INDENT * getattr(self, 'indent_level', 0)
        test = self.matcher.match_node(self.node.test, scope_parent_id=self.node_id).emit_expression()
        body = "\n".join([self.matcher.match_node(stmt, self.indent_level + 1, scope_parent_id=self.node_id).emit_statement() for stmt in self.node.body])

        if self.node.orelse:
            orelse = "\n".join([self.matcher.match_node(stmt, self.indent_level + 1, scope_parent_id=self.node_id).emit_statement() for stmt in self.node.orelse])
            return f"if {test} {{\n{body}\n{indent}}} else {{\n{orelse}\n{indent}}}"
        return f"{self.node_id} if {test} {{\n{body}\n{indent}}}"


class TsuchinokoFor(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.For)

    def emit_rust(self):
        target = self.matcher.match_node(self.node.target, scope_parent_id=self.node_id).emit_expression()
        iter_expr = self.matcher.match_node(self.node.iter, scope_parent_id=self.node_id).emit_expression()
        body = "\n".join(self.matcher.match_node(stmt, self.indent_level + 1, scope_parent_id=self.node_id).emit_statement() for stmt in self.node.body)
        return f"{self.node_id} for {target} in {iter_expr} {{\n{body}\n{INDENT * self.indent_level}}}"

class TsuchinokoTuple(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Tuple)

    def emit_rust(self):
        elements = [self.matcher.match_node(e, scope_parent_id=self.node_id).emit_expression() for e in self.node.elts]
        return f"({', '.join(elements)})"

class TsuchinokoList(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        # Pythonの list 構文 ([]) にマッチする
        return isinstance(node, ast.List)

    def emit_rust(self):
        return self.emit_expression()
    
    def emit_expression(self):
        elements = [self.matcher.match_node(e, scope_parent_id=self.node_id).emit_expression() for e in self.node.elts]
        return f"vec![{', '.join(elements)}]"

class TsuchinokoSubscript(TsuchinokoNode):
    # インデックスアクセス
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Subscript)

    def emit_rust(self):
        value = self.matcher.match_node(self.node.value, scope_parent_id=self.node_id).emit_expression()
        index = self.matcher.match_node(self.node.slice, scope_parent_id=self.node_id).emit_expression()
        return f"{value}[{index}]"

class TsuchinokoAttribute(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Attribute)

    def emit_rust(self):
        value = self.matcher.match_node(self.node.value, scope_parent_id=self.node_id).emit_expression()
        return f"{value}.{self.node.attr}"

class TsuchinokoPrint(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return (isinstance(node, ast.Call) and
                getattr(node.func, 'id', None) == 'print')

    def emit_rust(self):
        args = self.node.args
        fmt_parts = []
        expr_parts = []
        for arg in args:
            if isinstance(arg, ast.Constant) and isinstance(arg.value, str):
                fmt_parts.append(arg.value)
            else:
                value_expr = self.matcher.match_node(arg, scope_parent_id=self.node_id).emit_expression()
                print(value_expr)
                if "vec!" in value_expr or "Vec" in value_expr:
                    fmt_parts.append("{:?}")
                else:
                    print("なし")
                    fmt_parts.append("{}")
                expr_parts.append(value_expr)

        fmt_string = " ".join(fmt_parts)
        expr_string = ", ".join(expr_parts)
        if expr_parts:
            return f'println!("{fmt_string}", {expr_string});'
        else:
            return f'println!("{fmt_string}");'

    def emit_rust_old(self):
        args = self.node.args
        parts = []
        fmt = []
        for arg in args:
            if isinstance(arg, ast.Constant) and isinstance(arg.value, str):
                fmt.append(arg.value)
            else:
                fmt.append("{}")
                parts.append(self.matcher.match_node(arg, scope_parent_id=self.node_id).emit_expression())
        fmt_str = " ".join(fmt)
        args_str = ", ".join(parts)
        if parts:
            return f"println!(\"{fmt_str}\", {args_str})"
        else:
            return f"println!(\"{fmt_str}\")"

class TsuchinokoCallList(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Call) and getattr(node.func, 'id', '') == 'list'

    def emit_rust(self):
        arg = self.matcher.match_node(self.node.args[0], scope_parent_id=self.node_id).emit_expression()
        return f"Vec::from({arg})"

class TsuchinokoCallLen(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Call) and getattr(node.func, 'id', '') == 'len'

    def emit_rust(self):
        arg = self.matcher.match_node(self.node.args[0], scope_parent_id=self.node_id).emit_expression()
        return f"{arg}.len()"

class TsuchinokoContinue(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Continue)

    def emit_rust(self):
        return "continue;"

class TsuchinokoCallRange(TsuchinokoNode):
    @classmethod
    def matches(cls, node):
        return isinstance(node, ast.Call) and getattr(node.func, 'id', '') == 'range'

    def emit_rust(self):
        args = self.node.args
        if len(args) == 1:
            start = "0"
            end = self.matcher.match_node(args[0], scope_parent_id=self.node_id).emit_expression()
        elif len(args) == 2:
            start = self.matcher.match_node(args[0], scope_parent_id=self.node_id).emit_expression()
            end = self.matcher.match_node(args[1], scope_parent_id=self.node_id).emit_expression()
        elif len(args) == 3:
            start = self.matcher.match_node(args[0], scope_parent_id=self.node_id).emit_expression()
            end = self.matcher.match_node(args[1], scope_parent_id=self.node_id).emit_expression()
            step = self.matcher.match_node(args[2], scope_parent_id=self.node_id).emit_expression()
            return f"({start}..{end}).step_by({step} as usize)"
        return f"{start}..{end}"
