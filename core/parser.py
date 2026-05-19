# ─────────────────────────────────────────────
#  PenLang :: Parser
#  Добавить конструкцию: новый метод _parse_XXX, вызвать из _stmt() или _expr()
# ─────────────────────────────────────────────
from .lexer import Lexer, Token, TT
from .ast_nodes import *
from typing import List, Optional


class ParseError(Exception):
    def __init__(self, msg, token: Token):
        super().__init__(f"[Parser] {msg}  (line {token.line}, col {token.col})")


class Parser:
    def __init__(self, tokens: List[Token]):
        self.tokens = tokens
        self.pos    = 0

    # ── helpers ─────────────────────────────
    def _cur(self) -> Token:
        return self.tokens[self.pos]

    def _peek(self, offset=1) -> Token:
        p = self.pos + offset
        return self.tokens[p] if p < len(self.tokens) else self.tokens[-1]

    def _advance(self) -> Token:
        t = self.tokens[self.pos]
        if t.type != TT.EOF:
            self.pos += 1
        return t

    def _expect(self, tt: TT, value=None) -> Token:
        t = self._cur()
        if t.type != tt:
            raise ParseError(f"Expected {tt.name}, got {t.type.name} ({t.value!r})", t)
        if value is not None and t.value != value:
            raise ParseError(f"Expected {value!r}, got {t.value!r}", t)
        return self._advance()

    def _match(self, tt: TT, value=None) -> bool:
        t = self._cur()
        if t.type != tt:
            return False
        if value is not None and t.value != value:
            return False
        return True

    def _consume(self, tt: TT, value=None) -> Optional[Token]:
        if self._match(tt, value):
            return self._advance()
        return None

    def _is_kw(self, *words) -> bool:
        return self._cur().type == TT.KEYWORD and self._cur().value in words

    # ── public ──────────────────────────────
    def parse(self) -> Program:
        settings = {}
        body     = []

        # необязательная секция [setts]
        if self._match(TT.LBRACKET) and self._peek().type == TT.IDENT and self._peek().value == "setts":
            settings = self._parse_section("setts")

        # обязательная / единственная секция [scenary]
        self._expect(TT.LBRACKET)
        self._expect(TT.IDENT, "scenary")
        self._expect(TT.RBRACKET)

        while self._cur().type != TT.EOF:
            body.append(self._stmt())

        return Program(settings=settings, body=body)

    # ── sections ────────────────────────────
    def _parse_section(self, name: str) -> dict:
        self._expect(TT.LBRACKET)
        self._expect(TT.IDENT, name)
        self._expect(TT.RBRACKET)
        settings = {}
        while not self._match(TT.LBRACKET) and self._cur().type != TT.EOF:
            key = self._expect(TT.IDENT).value
            self._expect(TT.EQ)
            val = self._expr()
            self._expect(TT.SEMICOLON)
            settings[key] = val
        return settings

    # ── statements ──────────────────────────
    def _stmt(self) -> Node:
        t = self._cur()

        if self._is_kw("let"):    return self._let()
        if self._is_kw("use"):    return self._use()
        if self._is_kw("fn"):     return self._fn()
        if self._is_kw("return"): return self._return()
        if self._is_kw("if"):     return self._if()
        if self._is_kw("while"):  return self._while()
        if self._is_kw("for"):    return self._for()
        if self._is_kw("run"):    return self._run()
        if self._is_kw("log"):    return self._log()

        # augmented assign or expr-stmt
        return self._assign_or_expr()

    def _let(self) -> LetStmt:
        self._advance()                         # let
        name = self._expect(TT.IDENT).value
        type_name = None
        if self._consume(TT.COLON):
            type_name = self._expect(TT.IDENT).value
        self._expect(TT.EQ)
        value = self._expr()
        self._expect(TT.SEMICOLON)
        return LetStmt(name=name, value=value, type_name=type_name)

    def _use(self) -> ImportStmt:
        self._advance()                         # use
        path_tok = self._expect(TT.STRING)
        self._expect(TT.SEMICOLON)
        return ImportStmt(path=path_tok.value)

    def _fn(self) -> FunctionStmt:
        self._advance()                         # fn
        name = self._expect(TT.IDENT).value
        self._expect(TT.LPAREN)
        params = []
        while not self._match(TT.RPAREN):
            p_name = self._expect(TT.IDENT).value
            p_type = None
            if self._consume(TT.COLON):
                p_type = self._expect(TT.IDENT).value
            params.append((p_name, p_type))
            if not self._consume(TT.COMMA):
                break
        self._expect(TT.RPAREN)
        return_type = None
        if self._consume(TT.COLON):
            return_type = self._expect(TT.IDENT).value
        body = self._block()
        return FunctionStmt(name=name, params=params, body=body, return_type=return_type)

    def _return(self) -> ReturnStmt:
        self._advance()                         # return
        if self._match(TT.SEMICOLON):
            self._advance()
            return ReturnStmt(value=None)
        value = self._expr()
        self._expect(TT.SEMICOLON)
        return ReturnStmt(value=value)

    def _if(self) -> IfStmt:
        branches = []

        # if ( cond ) { body }
        self._advance()                         # if
        self._expect(TT.LPAREN)
        cond = self._expr()
        self._expect(TT.RPAREN)
        body = self._block()
        branches.append((cond, body))

        # elf ( cond ) { body }  — любое количество
        while self._is_kw("elf"):
            self._advance()
            self._expect(TT.LPAREN)
            cond = self._expr()
            self._expect(TT.RPAREN)
            body = self._block()
            branches.append((cond, body))

        # els { body }
        if self._is_kw("els"):
            self._advance()
            body = self._block()
            branches.append((None, body))

        return IfStmt(branches=branches)

    def _while(self) -> WhileStmt:
        self._advance()                         # while
        self._expect(TT.LPAREN)
        cond = self._expr()
        self._expect(TT.RPAREN)
        body = self._block()
        return WhileStmt(condition=cond, body=body)

    def _for(self) -> ForStmt:
        self._advance()                         # for
        self._expect(TT.LPAREN)
        var = self._expect(TT.IDENT).value
        self._expect(TT.KEYWORD, "in")
        iterable = self._expr()
        self._expect(TT.RPAREN)
        body = self._block()
        return ForStmt(var=var, iterable=iterable, body=body)

    def _run(self) -> RunStmt:
        self._advance()                         # run
        self._expect(TT.LPAREN)
        cmd = self._expr()
        self._expect(TT.RPAREN)
        self._expect(TT.SEMICOLON)
        return RunStmt(command=cmd)

    def _log(self) -> LogStmt:
        self._advance()                         # log
        self._expect(TT.LPAREN)
        val = self._expr()
        self._expect(TT.RPAREN)
        self._expect(TT.SEMICOLON)
        return LogStmt(value=val)

    def _assign_or_expr(self) -> Node:
        expr = self._expr()

        aug = {TT.PLUS_EQ:"+=", TT.MINUS_EQ:"-=",
               TT.STAR_EQ:"*=", TT.SLASH_EQ:"/=", TT.EQ:"="}
        if self._cur().type in aug:
            op  = aug[self._cur().type]
            self._advance()
            val = self._expr()
            self._expect(TT.SEMICOLON)
            return AssignStmt(target=expr, op=op, value=val)

        self._expect(TT.SEMICOLON)
        return ExprStmt(expr=expr)

    def _block(self) -> List[Node]:
        self._expect(TT.LBRACE)
        stmts = []
        while not self._match(TT.RBRACE) and self._cur().type != TT.EOF:
            stmts.append(self._stmt())
        self._expect(TT.RBRACE)
        return stmts

    # ── expressions (precedence climbing) ───
    def _expr(self) -> Node:
        return self._or()

    def _or(self) -> Node:
        left = self._and()
        while self._match(TT.OR):
            self._advance()
            left = BinOp(left=left, op="||", right=self._and())
        return left

    def _and(self) -> Node:
        left = self._compare()
        while self._match(TT.AND):
            self._advance()
            left = BinOp(left=left, op="&&", right=self._compare())
        return left

    def _compare(self) -> Node:
        left = self._add()
        ops  = {TT.EQEQ:"==", TT.NEQ:"!=", TT.LT:"<",
                TT.GT:">", TT.LTE:"<=", TT.GTE:">="}
        while self._cur().type in ops:
            op = ops[self._cur().type]
            self._advance()
            left = BinOp(left=left, op=op, right=self._add())
        return left

    def _add(self) -> Node:
        left = self._mul()
        while self._cur().type in (TT.PLUS, TT.MINUS):
            op = self._advance().value
            left = BinOp(left=left, op=op, right=self._mul())
        return left

    def _mul(self) -> Node:
        left = self._unary()
        while self._cur().type in (TT.STAR, TT.SLASH, TT.DOUBLESLASH,
                                    TT.PERCENT, TT.DOUBLESTAR):
            op = self._advance().value
            left = BinOp(left=left, op=op, right=self._unary())
        return left

    def _unary(self) -> Node:
        if self._match(TT.MINUS):
            self._advance()
            return UnaryOp(op="-", right=self._unary())
        if self._match(TT.NOT):
            self._advance()
            return UnaryOp(op="!", right=self._unary())
        return self._postfix()

    def _postfix(self) -> Node:
        node = self._primary()

        while True:
            # obj.field
            if self._match(TT.DOT):
                self._advance()
                field = self._expect(TT.IDENT).value
                node = AttrExpr(obj=node, name=field)

            # fn(...)
            elif self._match(TT.LPAREN):
                self._advance()

                args = []

                while not self._match(TT.RPAREN):
                    args.append(self._expr())

                    if not self._consume(TT.COMMA):
                        break

                self._expect(TT.RPAREN)
                node = CallExpr(callee=node, args=args)

            # obj[idx]
            elif self._match(TT.LBRACKET):
                self._advance()
                idx = self._expr()
                self._expect(TT.RBRACKET)

                node = IndexExpr(obj=node, index=idx)

            else:
                break

        return node

    def _primary(self) -> Node:
        t = self._cur()

        if t.type == TT.FLOAT:     self._advance(); return Literal(t.value)
        if t.type == TT.INT:
            self._advance()
            node = Literal(t.value)
            if self._match(TT.DOTDOT):
                self._advance()
                return RangeExpr(start=node, end=self._primary())
            return node
        if t.type == TT.STRING:    self._advance(); return Literal(t.value)
        if t.type == TT.FSTRING:   self._advance(); return FString(template=t.value)
        if t.type == TT.BOOL:      self._advance(); return Literal(t.value)
        if t.type == TT.NULL:      self._advance(); return Literal(None)

        # range  1..10  или просто ident
        if t.type == TT.IDENT:
            self._advance()
            node = Ident(name=t.value)
            if self._match(TT.DOTDOT):
                self._advance()
                return RangeExpr(start=node, end=self._primary())
            return node

        # number range  1..10
        # (уже обработано выше через _postfix для ident, но числа тоже могут быть)
        # grouped expr
        if t.type == TT.LPAREN:
            self._advance()
            node = self._expr()
            self._expect(TT.RPAREN)
            return node

        # list  [a, b, c]
        if t.type == TT.LBRACKET:
            return self._list()

        # json/dict  {k: v, ...}
        if t.type == TT.LBRACE:
            return self._json()

        raise ParseError(f"Unexpected token {t.type.name} ({t.value!r})", t)

    def _list(self) -> ListExpr:
        self._expect(TT.LBRACKET)
        items = []
        while not self._match(TT.RBRACKET):
            items.append(self._expr())
            if not self._consume(TT.COMMA):
                break
        self._expect(TT.RBRACKET)
        return ListExpr(items=items)

    def _json(self) -> JsonExpr:
        self._expect(TT.LBRACE)
        pairs = []
        while not self._match(TT.RBRACE):
            key = self._expr()
            self._expect(TT.COLON)
            val = self._expr()
            pairs.append((key, val))
            if not self._consume(TT.COMMA):
                break
        self._expect(TT.RBRACE)
        return JsonExpr(pairs=pairs)
