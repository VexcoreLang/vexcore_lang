# ─────────────────────────────────────────────
#  PenLang :: AST Nodes
#  Добавить узел: создай dataclass, используй в parser.py
# ─────────────────────────────────────────────
from dataclasses import dataclass, field
from typing import Any, List, Optional


# ── Базовый класс ────────────────────────────
class Node:
    pass


# ── Программа ────────────────────────────────
@dataclass
class Program(Node):
    settings: dict          # [setts] секция
    body:     List[Node]    # [scenary] секция


# ── Statements ───────────────────────────────
@dataclass
class LetStmt(Node):
    name:       str
    value:      Node
    type_name:  Optional[str] = None

@dataclass
class AssignStmt(Node):
    target: Node            # Ident или Index
    op:     str             # = += -= *= /=
    value:  Node

@dataclass
class IfStmt(Node):
    branches: List[tuple]   # [(condition, body), ...]  els → condition=None
    # branches[-1] может быть (None, body) для els

@dataclass
class WhileStmt(Node):
    condition: Node
    body:      List[Node]

@dataclass
class ForStmt(Node):
    var:      str
    iterable: Node
    body:     List[Node]

@dataclass
class RunStmt(Node):
    command: Node           # выражение → строка

@dataclass
class LogStmt(Node):
    value: Node

@dataclass
class ExprStmt(Node):       # выражение как statement (вызов функции и т.п.)
    expr: Node

@dataclass
class ImportStmt(Node):
    path: str

@dataclass
class FunctionStmt(Node):
    name:         str
    params:       List[tuple]      # [(name, optional_type), ...]
    body:         List[Node]
    return_type:  Optional[str] = None

@dataclass
class ReturnStmt(Node):
    value: Optional[Node] = None


# ── Expressions ──────────────────────────────
@dataclass
class BinOp(Node):
    left:  Node
    op:    str
    right: Node

@dataclass
class UnaryOp(Node):
    op:    str
    right: Node

@dataclass
class Ident(Node):
    name: str

@dataclass
class AttrExpr(Node):      # obj.name
    obj:  Node
    name: str

@dataclass
class Literal(Node):
    value: Any              # int | float | str | bool | None

@dataclass
class FString(Node):
    template: str           # "a = {a}" — интерполируется в интерпретаторе

@dataclass
class ListExpr(Node):
    items: List[Node]

@dataclass
class JsonExpr(Node):       # {key: value, ...}
    pairs: List[tuple]      # [(key_node, value_node), ...]

@dataclass
class IndexExpr(Node):      # expr[index]
    obj:   Node
    index: Node

@dataclass
class RangeExpr(Node):      # 1..10
    start: Node
    end:   Node

@dataclass
class CallExpr(Node):       # fn(args)  — для будущих функций
    callee: Node
    args:   List[Node]
