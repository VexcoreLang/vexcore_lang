use ast::{AssignOp, BinaryOp, Expr, Param, Program, Stmt, TypeAnnotation, UnaryOp, Value};
use lexer::{Token, TokenKind};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Expected {expected} at {line}:{col}, found {found}")]
    Expected {
        expected: String,
        found: String,
        line: usize,
        col: usize,
    },
    #[error("Invalid type annotation '{0}'")]
    InvalidType(String),
    #[error("Unexpected token {found} at {line}:{col}")]
    UnexpectedToken {
        found: String,
        line: usize,
        col: usize,
    },
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, ParseError> {
    Parser::new(tokens).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        while !self.is_eof() {
            statements.push(self.parse_stmt()?);
        }
        Ok(Program { statements })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match &self.peek().kind {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Fn => self.parse_fn_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Use => self.parse_use_stmt(),
            _ => self.parse_expr_or_assign_stmt(),
            // EXTENSION POINT: add new statement parser dispatch here.
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Let, "let")?;
        let name = self.expect_ident()?;
        let annotation = if self.matches(&TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(TokenKind::Assign, "=")?;
        let value = self.parse_expr()?;
        self.expect(TokenKind::Semicolon, ";")?;
        Ok(Stmt::Let {
            name,
            annotation,
            value,
        })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::If, "if")?;
        self.expect(TokenKind::LParen, "(")?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen, ")")?;
        let then_block = self.parse_block()?;

        let mut elif_blocks = Vec::new();
        while self.matches(&TokenKind::Elf) {
            self.expect(TokenKind::LParen, "(")?;
            let cond = self.parse_expr()?;
            self.expect(TokenKind::RParen, ")")?;
            let block = self.parse_block()?;
            elif_blocks.push((cond, block));
        }

        let else_block = if self.matches(&TokenKind::Els) {
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::If {
            cond,
            then_block,
            elif_blocks,
            else_block,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::While, "while")?;
        self.expect(TokenKind::LParen, "(")?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen, ")")?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::For, "for")?;
        self.expect(TokenKind::LParen, "(")?;
        let var = self.expect_ident()?;
        self.expect(TokenKind::In, "in")?;
        let start = self.parse_expr()?;
        if self.matches(&TokenKind::Range) {
            let end = self.parse_expr()?;
            self.expect(TokenKind::RParen, ")")?;
            let body = self.parse_block()?;
            Ok(Stmt::ForRange {
                var,
                start,
                end,
                body,
            })
        } else {
            self.expect(TokenKind::RParen, ")")?;
            let body = self.parse_block()?;
            Ok(Stmt::ForEach {
                var,
                iter: start,
                body,
            })
        }
    }

    fn parse_fn_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Fn, "fn")?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen, "(")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let pname = self.expect_ident()?;
                let annotation = if self.matches(&TokenKind::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                params.push(Param {
                    name: pname,
                    annotation,
                });
                if !self.matches(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, ")")?;
        let ret_type = if self.matches(&TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(Stmt::FnDef {
            name,
            params,
            ret_type,
            body,
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Return, "return")?;
        if self.matches(&TokenKind::Semicolon) {
            return Ok(Stmt::Return(None));
        }
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semicolon, ";")?;
        Ok(Stmt::Return(Some(expr)))
    }

    fn parse_use_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Use, "use")?;
        let tok = self.advance().clone();
        let path = match tok.kind {
            TokenKind::Str(s) => s,
            other => {
                return Err(ParseError::Expected {
                    expected: "string literal".to_string(),
                    found: token_name(&other),
                    line: tok.line,
                    col: tok.col,
                });
            }
        };

        let namespace = if self.matches(&TokenKind::As) {
            Some(self.expect_ident()?)
        } else {
            None
        };

        self.expect(TokenKind::Semicolon, ";")?;
        Ok(Stmt::Use { path, namespace })
    }

    fn parse_expr_or_assign_stmt(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expr()?;
        let op = if self.matches(&TokenKind::Assign) {
            Some(AssignOp::Assign)
        } else if self.matches(&TokenKind::PlusAssign) {
            Some(AssignOp::AddAssign)
        } else if self.matches(&TokenKind::MinusAssign) {
            Some(AssignOp::SubAssign)
        } else if self.matches(&TokenKind::StarAssign) {
            Some(AssignOp::MulAssign)
        } else if self.matches(&TokenKind::SlashAssign) {
            Some(AssignOp::DivAssign)
        } else {
            None
        };

        if let Some(op) = op {
            let value = self.parse_expr()?;
            self.expect(TokenKind::Semicolon, ";")?;
            Ok(Stmt::Assign {
                target: expr,
                op,
                value,
            })
        } else {
            self.expect(TokenKind::Semicolon, ";")?;
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect(TokenKind::LBrace, "{")?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace, "}")?;
        Ok(stmts)
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_and()?;
        while self.matches(&TokenKind::OrOr) {
            let right = self.parse_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_equality()?;
        while self.matches(&TokenKind::AndAnd) {
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = if self.matches(&TokenKind::EqEq) {
                Some(BinaryOp::Eq)
            } else if self.matches(&TokenKind::NotEq) {
                Some(BinaryOp::Ne)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_comparison()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;
        loop {
            let op = if self.matches(&TokenKind::Lt) {
                Some(BinaryOp::Lt)
            } else if self.matches(&TokenKind::Gt) {
                Some(BinaryOp::Gt)
            } else if self.matches(&TokenKind::Le) {
                Some(BinaryOp::Le)
            } else if self.matches(&TokenKind::Ge) {
                Some(BinaryOp::Ge)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_term()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = if self.matches(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.matches(&TokenKind::Minus) {
                Some(BinaryOp::Sub)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_factor()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_power()?;
        loop {
            let op = if self.matches(&TokenKind::Star) {
                Some(BinaryOp::Mul)
            } else if self.matches(&TokenKind::Slash) {
                Some(BinaryOp::Div)
            } else if self.matches(&TokenKind::FloorDiv) {
                Some(BinaryOp::FloorDiv)
            } else if self.matches(&TokenKind::Percent) {
                Some(BinaryOp::Mod)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_power()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        while self.matches(&TokenKind::Pow) {
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Pow,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.matches(&TokenKind::Not) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }
        if self.matches(&TokenKind::Minus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.matches(&TokenKind::LParen) {
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if !self.matches(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen, ")")?;
                expr = if let Expr::Var(name) = &expr {
                    if let Some((module, function)) = split_std_module_call(name) {
                        Expr::StdModuleCall {
                            module,
                            function,
                            args,
                        }
                    } else if let Some((module, function)) = split_module_call(name) {
                        Expr::ModuleCall {
                            module,
                            function,
                            args,
                        }
                    } else {
                        Expr::Call {
                            callee: Box::new(expr),
                            args,
                        }
                    }
                } else {
                    Expr::Call {
                        callee: Box::new(expr),
                        args,
                    }
                };
            } else if self.matches(&TokenKind::LBracket) {
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket, "]")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.matches(&TokenKind::Dot) {
                let name = self.expect_ident()?;
                expr = match expr {
                    Expr::Var(left) => Expr::Var(format!("{left}.{name}")),
                    _ => expr,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Int(v) => Ok(Expr::Value(Value::Int(v))),
            TokenKind::Float(v) => Ok(Expr::Value(Value::Float(v))),
            TokenKind::Str(s) => {
                if s.contains('{') && s.contains('}') {
                    Ok(Expr::InterpolatedString(s))
                } else {
                    Ok(Expr::Value(Value::Str(Arc::from(s))))
                }
            }
            TokenKind::True => Ok(Expr::Value(Value::Bool(true))),
            TokenKind::False => Ok(Expr::Value(Value::Bool(false))),
            TokenKind::Null => Ok(Expr::Value(Value::Null)),
            TokenKind::Ident(name) => Ok(Expr::Var(name)),
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen, ")")?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                let mut items = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        items.push(self.parse_expr()?);
                        if !self.matches(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket, "]")?;
                Ok(Expr::List(items))
            }
            TokenKind::LBrace => {
                let mut items = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    loop {
                        let key = match self.advance().kind.clone() {
                            TokenKind::Str(s) => s,
                            TokenKind::Ident(s) => s,
                            k => {
                                return Err(ParseError::Expected {
                                    expected: "json key".to_string(),
                                    found: token_name(&k),
                                    line: token.line,
                                    col: token.col,
                                });
                            }
                        };
                        self.expect(TokenKind::Colon, ":")?;
                        let value = self.parse_expr()?;
                        items.push((key, value));
                        if !self.matches(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBrace, "}")?;
                Ok(Expr::Json(items))
            }
            other => Err(ParseError::UnexpectedToken {
                found: token_name(&other),
                line: token.line,
                col: token.col,
            }),
        }
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, ParseError> {
        let ident = self.expect_ident()?;
        match ident.as_str() {
            "int" => Ok(TypeAnnotation::Int),
            "float" => Ok(TypeAnnotation::Float),
            "str" => Ok(TypeAnnotation::Str),
            "bool" => Ok(TypeAnnotation::Bool),
            "null" => Ok(TypeAnnotation::Null),
            "list" => Ok(TypeAnnotation::List),
            "json" => Ok(TypeAnnotation::Json),
            _ => Err(ParseError::InvalidType(ident)),
        }
    }

    fn expect(&mut self, kind: TokenKind, text: &str) -> Result<(), ParseError> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            let t = self.peek();
            Err(ParseError::Expected {
                expected: text.to_string(),
                found: token_name(&t.kind),
                line: t.line,
                col: t.col,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        let tk = self.advance().clone();
        if let TokenKind::Ident(s) = tk.kind {
            Ok(s)
        } else {
            Err(ParseError::Expected {
                expected: "identifier".to_string(),
                found: token_name(&tk.kind),
                line: tk.line,
                col: tk.col,
            })
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn matches(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_eof() {
            self.pos += 1;
        }
        self.prev()
    }

    fn prev(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }
}

fn split_module_call(name: &str) -> Option<(String, String)> {
    let mut parts = name.split('.');
    let a = parts.next()?;
    let b = parts.next()?;
    if parts.next().is_none() {
        Some((a.to_string(), b.to_string()))
    } else {
        None
    }
}

fn split_std_module_call(name: &str) -> Option<(String, String)> {
    let mut parts = name.split('.');
    let first = parts.next()?;
    if first != "std" {
        return None;
    }
    let module = parts.next()?;
    let function = parts.next()?;
    if parts.next().is_none() {
        Some((module.to_string(), function.to_string()))
    } else {
        None
    }
}

fn token_name(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Int(_) => "int".to_string(),
        TokenKind::Float(_) => "float".to_string(),
        TokenKind::Str(_) => "string".to_string(),
        TokenKind::Ident(s) => format!("ident({s})"),
        other => format!("{other:?}"),
    }
}
