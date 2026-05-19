# ─────────────────────────────────────────────
#  PenLang :: Interpreter
#  Добавить встроенную функцию: _builtins словарь внизу файла
#  Добавить новый тип: расширить _eval_binop / visit_Literal
# ─────────────────────────────────────────────
import subprocess
import re
import inspect
import os
from .ast_nodes import *


class RuntimeError_(Exception):
    def __init__(self, msg):
        super().__init__(f"[Runtime] {msg}")

class ReturnSignal(Exception):
    def __init__(self, value):
        self.value = value

class UserFunction:
    def __init__(self, node: FunctionStmt, closure: "Env"):
        self.node = node
        self.closure = closure


# ── Среда переменных ─────────────────────────
class Env:
    def __init__(self, parent=None):
        self._vars  = {}
        self._types = {}
        self.parent = parent

    def get(self, name: str):
        if name in self._vars:
            return self._vars[name]
        if self.parent:
            return self.parent.get(name)
        raise RuntimeError_(f"Undefined variable '{name}'")

    def set(self, name: str, value, type_name=None):
        self._vars[name] = value
        if type_name is not None:
            self._types[name] = type_name

    def get_type(self, name: str):
        if name in self._types:
            return self._types[name]
        if self.parent:
            return self.parent.get_type(name)
        return None

    def assign(self, name: str, value):
        """Ищет переменную вверх по цепочке и обновляет"""
        if name in self._vars:
            self._vars[name] = value
            return
        if self.parent:
            self.parent.assign(name, value)
            return
        raise RuntimeError_(f"Undefined variable '{name}' (use let first)")


# ── Интерпретатор ────────────────────────────
class Interpreter:
    def __init__(self, settings: dict = None):
        self.settings = settings or {}
        self.env      = Env()
        self._output  = []          # буфер вывода (log)
        self._imported_files = set()
        self._load_stdlib()

    # ── public ──────────────────────────────
    def run(self, program: Program):
        self.settings = {k: self._eval(v, self.env) for k, v in program.settings.items()}
        self._apply_settings()
        for stmt in program.body:
            self._exec(stmt, self.env)

    # ── settings ────────────────────────────
    def _apply_settings(self):
        # Здесь можно применить cpu/ram/mem ограничения
        # Например через resource модуль Python
        pass

    # ── execute statement ────────────────────
    def _exec(self, node: Node, env: Env):
        method = f"_exec_{type(node).__name__}"
        handler = getattr(self, method, None)
        if handler is None:
            raise RuntimeError_(f"Unknown statement: {type(node).__name__}")
        return handler(node, env)

    def _exec_LetStmt(self, node: LetStmt, env: Env):
        val = self._eval(node.value, env)
        if node.type_name:
            val = self._coerce_to_type(val, node.type_name, f"let {node.name}")
        env.set(node.name, val, type_name=node.type_name)

    def _exec_AssignStmt(self, node: AssignStmt, env: Env):
        val = self._eval(node.value, env)

        # простой идентификатор
        if isinstance(node.target, Ident):
            name = node.target.name
            if node.op == "=":
                expected_type = env.get_type(name)
                if expected_type:
                    val = self._coerce_to_type(val, expected_type, f"assign {name}")
                env.assign(name, val)
            else:
                cur = env.get(name)
                env.assign(name, self._augment(cur, node.op, val))
            return

        # индексация obj[key] = val
        if isinstance(node.target, IndexExpr):
            obj = self._eval(node.target.obj, env)
            key = self._eval(node.target.index, env)
            if node.op != "=":
                val = self._augment(obj[key], node.op, val)
            obj[key] = val
            return

        raise RuntimeError_("Invalid assignment target")

    def _exec_IfStmt(self, node: IfStmt, env: Env):
        for cond, body in node.branches:
            if cond is None or self._truthy(self._eval(cond, env)):
                child = Env(parent=env)
                for stmt in body:
                    self._exec(stmt, child)
                return

    def _exec_WhileStmt(self, node: WhileStmt, env: Env):
        while self._truthy(self._eval(node.condition, env)):
            child = Env(parent=env)
            for stmt in node.body:
                self._exec(stmt, child)

    def _exec_ForStmt(self, node: ForStmt, env: Env):
        iterable = self._eval(node.iterable, env)
        for item in iterable:
            child = Env(parent=env)
            child.set(node.var, item)
            for stmt in node.body:
                self._exec(stmt, child)

    def _exec_RunStmt(self, node: RunStmt, env: Env):
        cmd = self._eval(node.command, env)
        if not isinstance(cmd, str):
            raise RuntimeError_("run() argument must be a string")
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        if result.stdout:
            print(result.stdout, end="")
        if result.stderr:
            print(result.stderr, end="")

    def _exec_LogStmt(self, node: LogStmt, env: Env):
        val = self._eval(node.value, env)
        out = self._to_str(val)
        self._output.append(out)
        print(out)

    def _exec_ExprStmt(self, node: ExprStmt, env: Env):
        self._eval(node.expr, env)

    def _exec_ImportStmt(self, node: ImportStmt, env: Env):
        path = node.path
        if not path.endswith(".vcl"):
            path = path + ".vcl"
        full_path = os.path.abspath(path)
        if full_path in self._imported_files:
            return
        if not os.path.exists(full_path):
            raise RuntimeError_(f"Import file not found: {path}")
        self._imported_files.add(full_path)
        with open(full_path, encoding="utf-8") as f:
            source = f.read()
        from .lexer import Lexer
        from .parser import Parser
        imported_program = Parser(Lexer(source).tokenize()).parse()
        for stmt in imported_program.body:
            self._exec(stmt, self.env)

    def _exec_FunctionStmt(self, node: FunctionStmt, env: Env):
        env.set(node.name, UserFunction(node, env))

    def _exec_ReturnStmt(self, node: ReturnStmt, env: Env):
        value = None if node.value is None else self._eval(node.value, env)
        raise ReturnSignal(value)

    # ── evaluate expression ──────────────────
    def _eval(self, node: Node, env: Env):
        method = f"_eval_{type(node).__name__}"
        handler = getattr(self, method, None)
        if handler is None:
            raise RuntimeError_(f"Unknown expression: {type(node).__name__}")
        return handler(node, env)

    def _eval_Literal(self, node: Literal, env: Env):
        return node.value

    def _eval_Ident(self, node: Ident, env: Env):
        val = env.get(node.name)
        return val

    def _eval_AttrExpr(self, node: AttrExpr, env: Env):
        obj = self._eval(node.obj, env)
        if isinstance(obj, dict) and node.name in obj:
            return obj[node.name]
        raise RuntimeError_(f"Object has no attribute '{node.name}'")

    def _eval_FString(self, node: FString, env: Env):
        """Интерполирует {expr} в строке"""
        def replace(m):
            expr_src = m.group(1)
            # простой случай — имя переменной
            try:
                from .lexer import Lexer
                from .parser import Parser
                tokens = Lexer(expr_src).tokenize()
                ast    = Parser(tokens)._expr()
                return self._to_str(self._eval(ast, env))
            except Exception as e:
                raise RuntimeError_(f"f-string expression error: {e}")
        return re.sub(r'\{([^}]+)\}', replace, node.template)

    def _eval_BinOp(self, node: BinOp, env: Env):
        # короткое замыкание для && и ||
        if node.op == "&&":
            l = self._eval(node.left, env)
            return l if not self._truthy(l) else self._eval(node.right, env)
        if node.op == "||":
            l = self._eval(node.left, env)
            return l if self._truthy(l) else self._eval(node.right, env)

        l = self._eval(node.left,  env)
        r = self._eval(node.right, env)
        return self._apply_binop(l, node.op, r)

    def _eval_UnaryOp(self, node: UnaryOp, env: Env):
        v = self._eval(node.right, env)
        if node.op == "-": return -v
        if node.op == "!": return not self._truthy(v)
        raise RuntimeError_(f"Unknown unary op: {node.op}")

    def _eval_ListExpr(self, node: ListExpr, env: Env):
        return [self._eval(i, env) for i in node.items]

    def _eval_JsonExpr(self, node: JsonExpr, env: Env):
        result = {}
        for k, v in node.pairs:
            key = self._eval(k, env)
            val = self._eval(v, env)
            result[key] = val
        return result

    def _eval_IndexExpr(self, node: IndexExpr, env: Env):
        obj = self._eval(node.obj,   env)
        idx = self._eval(node.index, env)
        try:
            return obj[idx]
        except (KeyError, IndexError, TypeError) as e:
            raise RuntimeError_(f"Index error: {e}")

    def _eval_RangeExpr(self, node: RangeExpr, env: Env):
        start = self._eval(node.start, env)
        end   = self._eval(node.end,   env)
        return list(range(int(start), int(end) + 1))

    # ── helpers ─────────────────────────────
    def _apply_binop(self, l, op, r):
        try:
            if op == "+":   return l + r
            if op == "-":   return l - r
            if op == "*":   return l * r
            if op == "/":   return l / r
            if op == "//":  return l // r
            if op == "%":   return l % r
            if op == "**":  return l ** r
            if op == "==":  return l == r
            if op == "!=":  return l != r
            if op == "<":   return l < r
            if op == ">":   return l > r
            if op == "<=":  return l <= r
            if op == ">=":  return l >= r
        except TypeError as e:
            raise RuntimeError_(f"Type error in '{op}': {e}")
        raise RuntimeError_(f"Unknown operator: {op}")

    def _augment(self, cur, op, val):
        m = {"+=":(lambda a,b: a+b), "-=":(lambda a,b: a-b),
             "*=":(lambda a,b: a*b), "/=":(lambda a,b: a/b)}
        if op not in m:
            raise RuntimeError_(f"Unknown augmented operator: {op}")
        return m[op](cur, val)

    def _truthy(self, val) -> bool:
        if val is None:   return False
        if val is False:  return False
        if val == 0:      return False
        if val == "":     return False
        if isinstance(val, (list, dict)) and len(val) == 0: return False
        return True

    def _to_str(self, val) -> str:
        if val is None:  return "null"
        if val is True:  return "true"
        if val is False: return "false"
        if isinstance(val, dict):
            pairs = ", ".join(f"{self._to_str(k)}: {self._to_str(v)}" for k,v in val.items())
            return "{" + pairs + "}"
        if isinstance(val, list):
            return "[" + ", ".join(self._to_str(i) for i in val) + "]"
        return str(val)
    
    def _load_stdlib(self):
        import importlib, pkgutil, stdlib
        for finder, name, _ in pkgutil.iter_modules(stdlib.__path__):
            module = importlib.import_module(f"stdlib.{name}")
            if hasattr(module, "EXPORTS"):
                module_obj = {}
                for fn_name, fn in module.EXPORTS.items():
                    # Регистрация через пространство имён модуля: net.ping(...)
                    module_obj[fn_name] = ("__builtin__", fn)
                self.env.set(name, module_obj)

    def _eval_CallExpr(self, node: CallExpr, env: Env):
        fn = self._eval(node.callee, env)
        args = [self._eval(arg, env) for arg in node.args]

        if isinstance(fn, UserFunction):
            return self._call_user_function(fn, args)

        if not isinstance(fn, tuple) or fn[0] != "__builtin__":
            raise RuntimeError_("Value is not callable")

        _, callable_fn = fn

        # Совместимость:
        # 1) новый стиль: fn(*args)
        # 2) старый stdlib-стиль: fn(args, env)
        try:
            params = list(inspect.signature(callable_fn).parameters.values())
            if (
                len(params) == 2
                and params[0].kind in (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD)
                and params[1].kind in (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD)
            ):
                return callable_fn(args, env)
        except (TypeError, ValueError):
            # Для callables без доступной сигнатуры просто вызываем как fn(*args).
            pass

        return callable_fn(*args)

    def _call_user_function(self, fn: UserFunction, args: list):
        params = fn.node.params
        if len(args) != len(params):
            raise RuntimeError_(f"Function '{fn.node.name}' expects {len(params)} args, got {len(args)}")
        call_env = Env(parent=fn.closure)
        for i, (p_name, p_type) in enumerate(params):
            arg_val = args[i]
            if p_type:
                arg_val = self._coerce_to_type(arg_val, p_type, f"arg {p_name} in {fn.node.name}()")
            call_env.set(p_name, arg_val, type_name=p_type)
        try:
            for stmt in fn.node.body:
                self._exec(stmt, call_env)
            ret = None
        except ReturnSignal as r:
            ret = r.value
        if fn.node.return_type:
            ret = self._coerce_to_type(ret, fn.node.return_type, f"return of {fn.node.name}()")
        return ret

    def _coerce_to_type(self, value, type_name: str, where: str):
        t = type_name.lower()
        if t == "any":
            return value
        if t == "int":
            if isinstance(value, bool):
                return int(value)
            if isinstance(value, (int, float, str)):
                try:
                    return int(value)
                except Exception:
                    pass
            raise RuntimeError_(f"Type mismatch at {where}: expected int")
        if t == "float":
            if isinstance(value, bool):
                return float(value)
            if isinstance(value, (int, float, str)):
                try:
                    return float(value)
                except Exception:
                    pass
            raise RuntimeError_(f"Type mismatch at {where}: expected float")
        if t == "str":
            return self._to_str(value)
        if t == "bool":
            if isinstance(value, bool):
                return value
            if isinstance(value, (int, float)):
                return value != 0
            if isinstance(value, str):
                low = value.strip().lower()
                if low in ("true", "1", "yes", "on"):
                    return True
                if low in ("false", "0", "no", "off", ""):
                    return False
            return self._truthy(value)
        if t == "list":
            if isinstance(value, list):
                return value
            raise RuntimeError_(f"Type mismatch at {where}: expected list")
        if t == "json":
            if isinstance(value, dict):
                return value
            raise RuntimeError_(f"Type mismatch at {where}: expected json")
        if t == "null":
            if value is None:
                return None
            raise RuntimeError_(f"Type mismatch at {where}: expected null")
        raise RuntimeError_(f"Unknown type '{type_name}' at {where}")
