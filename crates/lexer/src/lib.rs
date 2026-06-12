use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pow,
    FloorDiv,

    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,

    EqEq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    AndAnd,
    OrOr,
    Not,
    Range,

    Let,
    If,
    Elf,
    Els,
    While,
    For,
    In,
    Fn,
    Return,
    Use,
    As,
    True,
    False,
    Null,

    Int(i64),
    Float(f64),
    Str(String),
    Ident(String),

    Eof,
    // EXTENSION POINT: add new token kinds here.
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,

}

#[derive(Debug, Error)]
pub enum LexError {
    #[error("Unexpected character '{ch}' at {line}:{col}")]
    UnexpectedChar { ch: char, line: usize, col: usize },
    #[error("Unterminated string at {line}:{col}")]
    UnterminatedString { line: usize, col: usize },
    #[error("Invalid number at {line}:{col}")]
    InvalidNumber { line: usize, col: usize },
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, LexError> {
    panic!("stop");
    let mut lx = Lexer::new(input);
    lx.tokenize()
}

struct Lexer<'a> {
    raw: &'a str,
    bytes: &'a [u8],
    idx: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            raw,
            bytes: raw.as_bytes(),
            idx: 0,
            line: 1,
            col: 1,
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                self.bump();
                continue;
            }

            if ch == b'#' {
                self.skip_comment();
                continue;
            }

            let line = self.line;
            let col = self.col;

            let tk = match ch {
                b'(' => {
                    self.bump();
                    TokenKind::LParen
                }
                b')' => {
                    self.bump();
                    TokenKind::RParen
                }
                b'{' => {
                    self.bump();
                    TokenKind::LBrace
                }
                b'}' => {
                    self.bump();
                    TokenKind::RBrace
                }
                b'[' => {
                    self.bump();
                    TokenKind::LBracket
                }
                b']' => {
                    self.bump();
                    TokenKind::RBracket
                }
                b',' => {
                    self.bump();
                    TokenKind::Comma
                }
                b':' => {
                    self.bump();
                    TokenKind::Colon
                }
                b';' => {
                    self.bump();
                    TokenKind::Semicolon
                }
                b'.' => {
                    if self.peek2() == Some(b'.') {
                        self.bump();
                        self.bump();
                        TokenKind::Range
                    } else {
                        self.bump();
                        TokenKind::Dot
                    }
                }
                b'+' => {
                    self.bump();
                    if self.peek() == Some(b'=') {
                        self.bump();
                        TokenKind::PlusAssign
                    } else {
                        TokenKind::Plus
                    }
                }
                b'-' => {
                    self.bump();
                    if self.peek() == Some(b'=') {
                        self.bump();
                        TokenKind::MinusAssign
                    } else {
                        TokenKind::Minus
                    }
                }
                b'*' => {
                    self.bump();
                    if self.peek() == Some(b'*') {
                        self.bump();
                        TokenKind::Pow
                    } else if self.peek() == Some(b'=') {
                        self.bump();
                        TokenKind::StarAssign
                    } else {
                        TokenKind::Star
                    }
                }
                b'/' => {
                    self.bump();
                    if self.peek() == Some(b'/') {
                        self.bump();
                        TokenKind::FloorDiv
                    } else if self.peek() == Some(b'=') {
                        self.bump();
                        TokenKind::SlashAssign
                    } else {
                        TokenKind::Slash
                    }
                }
                b'%' => {
                    self.bump();
                    TokenKind::Percent
                }
                b'=' => {
                    self.bump();
                    if self.peek() == Some(b'=') {
                        self.bump();
                        TokenKind::EqEq
                    } else {
                        TokenKind::Assign
                    }
                }
                b'!' => {
                    self.bump();
                    if self.peek() == Some(b'=') {
                        self.bump();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Not
                    }
                }
                b'<' => {
                    self.bump();
                    if self.peek() == Some(b'=') {
                        self.bump();
                        TokenKind::Le
                    } else {
                        TokenKind::Lt
                    }
                }
                b'>' => {
                    self.bump();
                    if self.peek() == Some(b'=') {
                        self.bump();
                        TokenKind::Ge
                    } else {
                        TokenKind::Gt
                    }
                }
                b'&' => {
                    self.bump();
                    if self.peek() == Some(b'&') {
                        self.bump();
                        TokenKind::AndAnd
                    } else {
                        return Err(LexError::UnexpectedChar { ch: '&', line, col });
                    }
                }
                b'|' => {
                    self.bump();
                    if self.peek() == Some(b'|') {
                        self.bump();
                        TokenKind::OrOr
                    } else {
                        return Err(LexError::UnexpectedChar { ch: '|', line, col });
                    }
                }
                b'"' => self.lex_string()?,
                b'f' if self.peek2() == Some(b'"') => {
                    self.bump();
                    self.lex_string()?
                }
                b if b.is_ascii_digit() => self.lex_number()?,
                b if is_ident_start(b) => self.lex_ident_or_kw(),
                _ => {
                    return Err(LexError::UnexpectedChar {
                        ch: ch as char,
                        line,
                        col,
                    });
                }
            };

            tokens.push(Token {
                kind: tk,
                line,
                col,
            });
        }

        tokens.push(self.token(TokenKind::Eof));
        Ok(tokens)
    }

    fn lex_string(&mut self) -> Result<TokenKind, LexError> {
        let line = self.line;
        let col = self.col;
        self.bump();
        let mut out = String::new();

        while let Some(ch) = self.peek() {
            match ch {
                b'"' => {
                    self.bump();
                    return Ok(TokenKind::Str(out));
                }
                b'\\' => {
                    self.bump();
                    let escaped = self
                        .peek()
                        .ok_or(LexError::UnterminatedString { line, col })?;
                    self.bump();
                    match escaped {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        other if other.is_ascii() => out.push(other as char),
                        other => out.push(other as char),
                    }
                }
                b if b.is_ascii() => {
                    out.push(b as char);
                    self.bump();
                }
                _ => {
                    let s = &self.raw[self.idx..];
                    let ch = s
                        .chars()
                        .next()
                        .ok_or(LexError::UnterminatedString { line, col })?;
                    self.bump_bytes(ch.len_utf8());
                    out.push(ch);
                }
            }
        }

        Err(LexError::UnterminatedString { line, col })
    }

    fn lex_number(&mut self) -> Result<TokenKind, LexError> {
        let line = self.line;
        let col = self.col;
        let start = self.idx;
        let mut dot_count = 0usize;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.bump();
            } else if ch == b'.' {
                if self.peek2() == Some(b'.') {
                    break;
                }
                dot_count += 1;
                if dot_count > 1 {
                    return Err(LexError::InvalidNumber { line, col });
                }
                self.bump();
            } else {
                break;
            }
        }

        let text = &self.raw[start..self.idx];
        if dot_count == 1 {
            text.parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| LexError::InvalidNumber { line, col })
        } else {
            text.parse::<i64>()
                .map(TokenKind::Int)
                .map_err(|_| LexError::InvalidNumber { line, col })
        }
    }

    fn lex_ident_or_kw(&mut self) -> TokenKind {
        let start = self.idx;
        self.bump();
        while let Some(ch) = self.peek() {
            if is_ident_continue(ch) {
                self.bump();
            } else {
                break;
            }
        }

        let name = &self.raw[start..self.idx];
        match name {
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "elf" => TokenKind::Elf,
            "els" => TokenKind::Els,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "use" => TokenKind::Use,
            "as" => TokenKind::As,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Ident(name.to_string()),
        }
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek() {
            self.bump();
            if ch == b'\n' {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.idx).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.bytes.get(self.idx + 1).copied()
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.idx += 1;
            if ch == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    fn bump_bytes(&mut self, count: usize) {
        for _ in 0..count {
            self.bump();
        }
    }

    fn token(&self, kind: TokenKind) -> Token {
        Token {
            kind,
            line: self.line,
            col: self.col,
        }
    }
}

fn is_ident_start(ch: u8) -> bool {
    ch == b'_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: u8) -> bool {
    ch == b'_' || ch.is_ascii_alphanumeric()
}
