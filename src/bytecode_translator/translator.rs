use ast::{AssignOp, BinaryOp, Expr, Param, Program, Stmt, TypeAnnotation, UnaryOp, Value};
use std::fs;
use std::path::Path;
use thiserror::Error;
use crate::bytecode::{Bytecode, CompiledFunction, InterpPart, OpCode};

#[derive(Debug, Error)]
pub enum BytecodeTranslatorError {
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub struct BytecodeTranslator;

impl BytecodeTranslator {
    pub fn new() -> Self {
        Self
    }

    pub fn translate_program(&mut self, program: &Program) -> Result<Bytecode, BytecodeTranslatorError> {
        self.translate_stmts(&program.statements)
    }

    fn translate_stmts(&mut self, stmts: &[Stmt]) -> Result<Bytecode, BytecodeTranslatorError> {
        let mut code = Bytecode::new();
        for stmt in stmts {
            self.translate_stmt(stmt, &mut code)?;
        }
        Ok(code)
    }

    fn translate_scoped_stmts(&mut self, stmts: &[Stmt]) -> Result<Bytecode, BytecodeTranslatorError> {
        self.translate_stmts(stmts)
    }

    fn translate_stmt(&mut self, stmt: &Stmt, code: &mut Bytecode) -> Result<(), BytecodeTranslatorError> {
        match stmt {
            Stmt::Let { name, annotation, value } => {
                self.translate_expr(value, code)?;
                if let Some(ann) = annotation {
                    code.code.push(OpCode::Cast(*ann));
                }
                code.code.push(OpCode::StoreLocal(name.clone()));
            }
            Stmt::Assign { target, op, value } => match target {
                Expr::Var(name) => {
                    self.translate_expr(value, code)?;
                    code.code.push(OpCode::UpdateLocal { name: name.clone(), op: *op });
                }
                _ => return Err(BytecodeTranslatorError::InvalidOperation(
                    "assignment target must be a variable".to_string(),
                )),
            },
            Stmt::If { cond, then_block, elif_blocks, else_block } => {
                let then_code = self.translate_scoped_stmts(then_block)?;
                let mut compiled_elifs = Vec::with_capacity(elif_blocks.len());
                for (elif_cond, block) in elif_blocks {
                    compiled_elifs.push((self.translate_expr_chunk(elif_cond)?, self.translate_scoped_stmts(block)?));
                }
                let else_code = match else_block {
                    Some(block) => Some(self.translate_scoped_stmts(block)?),
                    None => None,
                };
                code.code.push(OpCode::If {
                    cond: self.translate_expr_chunk(cond)?,
                    then_body: then_code,
                    elifs: compiled_elifs,
                    else_body: else_code,
                });
            }
            Stmt::While { cond, body } => {
                code.code.push(OpCode::While {
                    cond: self.translate_expr_chunk(cond)?,
                    body: self.translate_scoped_stmts(body)?,
                });
            }
            Stmt::ForRange { var, start, end, body } => {
                code.code.push(OpCode::ForRange {
                    start: self.translate_expr_chunk(start)?,
                    end: self.translate_expr_chunk(end)?,
                    var: var.clone(),
                    body: self.translate_scoped_stmts(body)?,
                });
            }
            Stmt::ForEach { var, iter, body } => {
                code.code.push(OpCode::ForEach {
                    iter: self.translate_expr_chunk(iter)?,
                    var: var.clone(),
                    body: self.translate_scoped_stmts(body)?,
                });
            }
            Stmt::FnDef { name, params, ret_type, body } => {
                let function = CompiledFunction {
                    params: params.to_vec(),
                    ret_type: *ret_type,
                    chunk: self.translate_stmts(body)?,
                };
                code.code.push(OpCode::DefineFunction { name: name.clone(), function });
            }
            Stmt::Return(expr) => {
                let compiled = match expr {
                    Some(expr) => Some(self.translate_expr_chunk(expr)?),
                    None => None,
                };
                code.code.push(OpCode::Return(compiled));
            }
            Stmt::Use { path, namespace } => code.code.push(OpCode::Use { path: path.clone(), namespace: namespace.clone() }),
            Stmt::Expr(expr) => {
                self.translate_expr(expr, code)?;
                code.code.push(OpCode::Pop);
            }
        }
        Ok(())
    }

    fn translate_expr_chunk(&mut self, expr: &Expr) -> Result<Bytecode, BytecodeTranslatorError> {
        let mut code = Bytecode::new();
        self.translate_expr(expr, &mut code)?;
        Ok(code)
    }

    fn translate_expr(&mut self, expr: &Expr, code: &mut Bytecode) -> Result<(), BytecodeTranslatorError> {
        match expr {
            Expr::Value(v) => code.code.push(OpCode::Const(v.clone())),
            Expr::Var(name) => code.code.push(OpCode::LoadLocal(name.clone())),
            Expr::InterpolatedString(raw) => code.code.push(OpCode::Interpolate(parse_interpolated_parts(raw))),
            Expr::List(items) => {
                for item in items {
                    self.translate_expr(item, code)?;
                }
                code.code.push(OpCode::BuildList(items.len()));
            }
            Expr::Json(entries) => {
                let mut keys = Vec::with_capacity(entries.len());
                for (key, value) in entries {
                    keys.push(key.clone());
                    self.translate_expr(value, code)?;
                }
                code.code.push(OpCode::BuildMap(keys));
            }
            Expr::Unary { op, expr } => {
                self.translate_expr(expr, code)?;
                code.code.push(OpCode::Unary(*op));
            }
            Expr::Binary { left, op, right } => {
                self.translate_expr(left, code)?;
                self.translate_expr(right, code)?;
                code.code.push(OpCode::Binary(*op));
            }
            Expr::Call { callee, args } => {
                let name = match &**callee {
                    Expr::Var(name) => name.clone(),
                    _ => return Err(BytecodeTranslatorError::InvalidOperation(
                        "call target must be an identifier".to_string(),
                    )),
                };
                for arg in args {
                    self.translate_expr(arg, code)?;
                }
                code.code.push(OpCode::CallLocal { name, argc: args.len() });
            }
            Expr::ModuleCall { module, function, args } => {
                for arg in args {
                    self.translate_expr(arg, code)?;
                }
                code.code.push(OpCode::CallModule { module: module.clone(), function: function.clone(), argc: args.len() });
            }
            Expr::StdModuleCall { module, function, args } => {
                for arg in args {
                    self.translate_expr(arg, code)?;
                }
                code.code.push(OpCode::CallStd { module: module.clone(), function: function.clone(), argc: args.len() });
            }
            Expr::Index { object, index } => {
                self.translate_expr(object, code)?;
                self.translate_expr(index, code)?;
                code.code.push(OpCode::IndexGet);
            }
        }
        Ok(())
    }
}

pub fn save_bytecode(code: &Bytecode, path: &Path) -> Result<(), BytecodeTranslatorError> {
    let data = serde_json::to_vec_pretty(code).map_err(|e| BytecodeTranslatorError::Serialization(e.to_string()))?;
    fs::write(path, data).map_err(|e| BytecodeTranslatorError::Io(e.to_string()))
}

pub fn load_bytecode(path: &Path) -> Result<Bytecode, BytecodeTranslatorError> {
    let data = fs::read(path).map_err(|e| BytecodeTranslatorError::Io(e.to_string()))?;
    serde_json::from_slice(&data).map_err(|e| BytecodeTranslatorError::Serialization(e.to_string()))
}

fn parse_interpolated_parts(raw: &str) -> Vec<InterpPart> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if !current.is_empty() {
                parts.push(InterpPart::Text(std::mem::take(&mut current)));
            }
            let mut var = String::new();
            while let Some(next) = chars.next() {
                if next == '}' {
                    break;
                }
                var.push(next);
            }
            if !var.is_empty() {
                parts.push(InterpPart::Var(var));
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        parts.push(InterpPart::Text(current));
    }

    parts
}
