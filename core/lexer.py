# ─────────────────────────────────────────────
#  PenLang :: Lexer
#  Добавить токен: 1) TOKEN_TYPES  2) KEYWORDS или regex в _rules()
# ─────────────────────────────────────────────
import re
from dataclasses import dataclass
from enum import Enum, auto
from typing import List


class TT(Enum):
    # literals
    INT        = auto()
    FLOAT      = auto()
    STRING     = auto()
    FSTRING    = auto()
    BOOL       = auto()
    NULL       = auto()
    # identifiers / keywords
    IDENT      = auto()
    KEYWORD    = auto()
    # operators
    PLUS       = auto()
    MINUS      = auto()
    STAR       = auto()
    SLASH      = auto()
    DOUBLESLASH= auto()
    PERCENT    = auto()
    DOUBLESTAR = auto()
    # assign / augmented assign
    EQ         = auto()
    PLUS_EQ    = auto()
    MINUS_EQ   = auto()
    STAR_EQ    = auto()
    SLASH_EQ   = auto()
    # comparison / logical
    EQEQ       = auto()
    NEQ        = auto()
    LT         = auto()
    GT         = auto()
    LTE        = auto()
    GTE        = auto()
    AND        = auto()
    OR         = auto()
    NOT        = auto()
    # punctuation
    LPAREN     = auto()
    RPAREN     = auto()
    LBRACE     = auto()
    RBRACE     = auto()
    LBRACKET   = auto()
    RBRACKET   = auto()
    COLON      = auto()
    COMMA      = auto()
    DOT        = auto()
    DOTDOT     = auto()   # range  1..10
    SEMICOLON  = auto()
    # misc
    EOF        = auto()


KEYWORDS = {
    "let", "if", "elf", "els", "while", "for", "in",
    "run", "log", "true", "false", "null",
    "fn", "return", "use", "break", "continue",   # зарезервировано для будущего
}


@dataclass
class Token:
    type:  TT
    value: object
    line:  int
    col:   int

    def __repr__(self):
        return f"Token({self.type.name}, {self.value!r}, {self.line}:{self.col})"


class LexerError(Exception):
    def __init__(self, msg, line, col):
        super().__init__(f"[Lexer] {msg}  (line {line}, col {col})")


class Lexer:
    def __init__(self, source: str):
        self.src   = source
        self.pos   = 0
        self.line  = 1
        self.col   = 1
        self.tokens: List[Token] = []

    # ── public ──────────────────────────────
    def tokenize(self) -> List[Token]:
        while self.pos < len(self.src):
            self._next()
        self.tokens.append(Token(TT.EOF, None, self.line, self.col))
        return self.tokens

    # ── internal ────────────────────────────
    def _cur(self):
        return self.src[self.pos] if self.pos < len(self.src) else ""

    def _peek(self, offset=1):
        p = self.pos + offset
        return self.src[p] if p < len(self.src) else ""

    def _advance(self, n=1):
        for _ in range(n):
            if self.pos < len(self.src):
                if self.src[self.pos] == "\n":
                    self.line += 1
                    self.col   = 1
                else:
                    self.col  += 1
                self.pos += 1

    def _add(self, tt, value):
        self.tokens.append(Token(tt, value, self.line, self.col))

    def _next(self):
        c = self._cur()

        # whitespace
        if c in " \t\r\n":
            self._advance()
            return

        # line comment
        if c == "#":
            while self.pos < len(self.src) and self.src[self.pos] != "\n":
                self._advance()
            return

        # f-string  f"..."
        if c == "f" and self._peek() in ('"', "'"):
            self._read_fstring()
            return

        # string
        if c in ('"', "'"):
            self._read_string()
            return

        # number
        if c.isdigit():
            self._read_number()
            return

        # identifier / keyword
        if c.isalpha() or c == "_":
            self._read_ident()
            return

        # two-char operators first
        two = c + self._peek()
        two_map = {
            "..": (TT.DOTDOT,      ".."),
            "//": (TT.DOUBLESLASH, "//"),
            "**": (TT.DOUBLESTAR,  "**"),
            "==": (TT.EQEQ,        "=="),
            "!=": (TT.NEQ,         "!="),
            "<=": (TT.LTE,         "<="),
            ">=": (TT.GTE,         ">="),
            "+=": (TT.PLUS_EQ,     "+="),
            "-=": (TT.MINUS_EQ,    "-="),
            "*=": (TT.STAR_EQ,     "*="),
            "/=": (TT.SLASH_EQ,    "/="),
            "||": (TT.OR,          "||"),
            "&&": (TT.AND,         "&&"),
        }
        if two in two_map:
            tt, val = two_map[two]
            self._add(tt, val)
            self._advance(2)
            return

        # single-char operators
        one_map = {
            "+": TT.PLUS,   "-": TT.MINUS,  "*": TT.STAR,
            "/": TT.SLASH,  "%": TT.PERCENT,
            "=": TT.EQ,     "<": TT.LT,     ">": TT.GT,
            "!": TT.NOT,
            "(": TT.LPAREN, ")": TT.RPAREN,
            "{": TT.LBRACE, "}": TT.RBRACE,
            "[": TT.LBRACKET, "]": TT.RBRACKET,
            ":": TT.COLON,  ",": TT.COMMA,
            ".": TT.DOT,    ";": TT.SEMICOLON,
        }
        if c in one_map:
            self._add(one_map[c], c)
            self._advance()
            return

        raise LexerError(f"Unexpected character {c!r}", self.line, self.col)

    def _read_number(self):
        start = self.pos
        is_float = False
        while self._cur().isdigit():
            self._advance()
        if self._cur() == "." and self._peek().isdigit():
            is_float = True
            self._advance()
            while self._cur().isdigit():
                self._advance()
        raw = self.src[start:self.pos]
        if is_float:
            self._add(TT.FLOAT, float(raw))
        else:
            self._add(TT.INT, int(raw))

    def _read_ident(self):
        start = self.pos
        while self._cur().isalnum() or self._cur() == "_":
            self._advance()
        word = self.src[start:self.pos]
        if word == "true":
            self._add(TT.BOOL, True)
        elif word == "false":
            self._add(TT.BOOL, False)
        elif word == "null":
            self._add(TT.NULL, None)
        elif word in KEYWORDS:
            self._add(TT.KEYWORD, word)
        else:
            self._add(TT.IDENT, word)

    def _read_string(self):
        quote = self._cur()
        self._advance()
        buf = []
        while self.pos < len(self.src) and self._cur() != quote:
            if self._cur() == "\\" :
                self._advance()
                esc = {"n":"\n","t":"\t","\\":"\\",'"':'"',"'":"'"}.get(self._cur(), self._cur())
                buf.append(esc)
            else:
                buf.append(self._cur())
            self._advance()
        if self.pos >= len(self.src):
            raise LexerError("Unterminated string", self.line, self.col)
        self._advance()  # closing quote
        self._add(TT.STRING, "".join(buf))

    def _read_fstring(self):
        self._advance()  # skip 'f'
        quote = self._cur()
        self._advance()  # skip quote
        buf = []
        while self.pos < len(self.src) and self._cur() != quote:
            buf.append(self._cur())
            self._advance()
        if self.pos >= len(self.src):
            raise LexerError("Unterminated f-string", self.line, self.col)
        self._advance()
        self._add(TT.FSTRING, "".join(buf))
