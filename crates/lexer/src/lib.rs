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
    let mut lx = Lexer::new(input);
    lx.tokenize()
}

struct Lexer<'a> {
    chars: Vec<char>,
    idx: usize,
    line: usize,
    col: usize,
    _raw: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            chars: raw.chars().collect(),
            idx: 0,
            line: 1,
            col: 1,
            _raw: raw,
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.bump();
                continue;
            }

            if ch == '#' {
                self.skip_comment();
                continue;
            }

            let line = self.line;
            let col = self.col;

            let tk = match ch {
                '(' => {
                    self.bump();
                    TokenKind::LParen
                }
                ')' => {
                    self.bump();
                    TokenKind::RParen
                }
                '{' => {
                    self.bump();
                    TokenKind::LBrace
                }
                '}' => {
                    self.bump();
                    TokenKind::RBrace
                }
                '[' => {
                    self.bump();
                    TokenKind::LBracket
                }
                ']' => {
                    self.bump();
                    TokenKind::RBracket
                }
                ',' => {
                    self.bump();
                    TokenKind::Comma
                }
                ':' => {
                    self.bump();
                    TokenKind::Colon
                }
                ';' => {
                    self.bump();
                    TokenKind::Semicolon
                }
                '.' => {
                    if self.peek2() == Some('.') {
                        self.bump();
                        self.bump();
                        TokenKind::Range
                    } else {
                        self.bump();
                        TokenKind::Dot
                    }
                }
                '+' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::PlusAssign
                    } else {
                        TokenKind::Plus
                    }
                }
                '-' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::MinusAssign
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.bump();
                    if self.peek() == Some('*') {
                        self.bump();
                        TokenKind::Pow
                    } else if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::StarAssign
                    } else {
                        TokenKind::Star
                    }
                }
                '/' => {
                    self.bump();
                    if self.peek() == Some('/') {
                        self.bump();
                        TokenKind::FloorDiv
                    } else if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::SlashAssign
                    } else {
                        TokenKind::Slash
                    }
                }
                '%' => {
                    self.bump();
                    TokenKind::Percent
                }
                '=' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::EqEq
                    } else {
                        TokenKind::Assign
                    }
                }
                '!' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Not
                    }
                }
                '<' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::Le
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::Ge
                    } else {
                        TokenKind::Gt
                    }
                }
                '&' => {
                    self.bump();
                    if self.peek() == Some('&') {
                        self.bump();
                        TokenKind::AndAnd
                    } else {
                        return Err(LexError::UnexpectedChar { ch: '&', line, col });
                    }
                }
                '|' => {
                    self.bump();
                    if self.peek() == Some('|') {
                        self.bump();
                        TokenKind::OrOr
                    } else {
                        return Err(LexError::UnexpectedChar { ch: '|', line, col });
                    }
                }
                '"' => self.lex_string()?,
                'f' if self.peek2() == Some('"') => {
                    self.bump();
                    self.lex_string()?
                }
                ch if ch.is_ascii_digit() => self.lex_number()?,
                ch if is_ident_start(ch) => self.lex_ident_or_kw(),
                _ => {
                    return Err(LexError::UnexpectedChar { ch, line, col });
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
                '"' => {
                    self.bump();
                    return Ok(TokenKind::Str(out));
                }
                '\\' => {
                    self.bump();
                    let escaped = self
                        .peek()
                        .ok_or(LexError::UnterminatedString { line, col })?;
                    self.bump();
                    match escaped {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        other => out.push(other),
                    }
                }
                _ => {
                    out.push(ch);
                    self.bump();
                }
            }
        }

        Err(LexError::UnterminatedString { line, col })
    }

    fn lex_number(&mut self) -> Result<TokenKind, LexError> {
        let line = self.line;
        let col = self.col;
        let mut num = String::new();
        let mut dot_count = 0usize;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num.push(ch);
                self.bump();
            } else if ch == '.' {
                if self.peek2() == Some('.') {
                    break;
                }
                dot_count += 1;
                if dot_count > 1 {
                    return Err(LexError::InvalidNumber { line, col });
                }
                num.push(ch);
                self.bump();
            } else {
                break;
            }
        }

        if dot_count == 1 {
            num.parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| LexError::InvalidNumber { line, col })
        } else {
            num.parse::<i64>()
                .map(TokenKind::Int)
                .map_err(|_| LexError::InvalidNumber { line, col })
        }
    }

    fn lex_ident_or_kw(&mut self) -> TokenKind {
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if is_ident_continue(ch) {
                name.push(ch);
                self.bump();
            } else {
                break;
            }
        }

        match name.as_str() {
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
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Ident(name),
        }
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek() {
            self.bump();
            if ch == '\n' {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.idx + 1).copied()
    }

    fn bump(&mut self) {
        if let Some(ch) = self.chars.get(self.idx).copied() {
            self.idx += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
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

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
