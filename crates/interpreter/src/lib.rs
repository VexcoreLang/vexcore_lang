use ast::{AssignOp, BinaryOp, Expr, Param, Program, Stmt, TypeAnnotation, UnaryOp, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),
    #[error("Type error: {0}")]
    TypeError(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Module error: {0}")]
    ModuleError(String),
    #[error("Import error: {0}")]
    ImportError(String),
    #[error("Parse during import failed: {0}")]
    ParseError(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CompiledFunction {
    params: Vec<Param>,
    ret_type: Option<TypeAnnotation>,
    code: Vec<Instr>,
}

#[derive(Clone)]
struct ModuleState {
    scopes: Vec<ScopeFrame>,
    functions: HashMap<String, CompiledFunction>,
    user_modules: HashMap<String, ModuleState>,
    base_dir: PathBuf,
}

#[derive(Clone, Default)]
struct ScopeFrame {
    names: HashMap<String, usize>,
    values: Vec<Value>,
}

impl ScopeFrame {
    fn new() -> Self {
        Self::default()
    }

    fn define(&mut self, name: String, value: Value) {
        if let Some(idx) = self.names.get(&name).copied() {
            self.values[idx] = value;
            return;
        }
        let idx = self.values.len();
        self.values.push(value);
        self.names.insert(name, idx);
    }

    fn set(&mut self, name: &str, value: Value) -> bool {
        if let Some(idx) = self.names.get(name).copied() {
            self.values[idx] = value;
            true
        } else {
            false
        }
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.names.get(name).and_then(|idx| self.values.get(*idx))
    }
}

#[derive(Clone)]
struct CachedImport {
    modified: Option<SystemTime>,
    program: Program,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum InterpPart {
    Text(String),
    Var(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Instr {
    Push(Value),
    LoadVar(String),
    Unary(UnaryOp),
    Binary(BinaryOp),
    MakeList(usize),
    MakeJson(Vec<String>),
    Interpolate(Vec<InterpPart>),
    CallFunction {
        name: String,
        argc: usize,
    },
    CallModule {
        module: String,
        function: String,
        argc: usize,
    },
    CallStdModule {
        module: String,
        function: String,
        argc: usize,
    },
    GetIndex,
    Coerce(TypeAnnotation),
    Pop,
    DefineVar(String),
    AssignVar {
        name: String,
        op: AssignOp,
    },
    DefineFunction {
        name: String,
        function: CompiledFunction,
    },
    If {
        cond: Vec<Instr>,
        then_body: Vec<Instr>,
        elifs: Vec<(Vec<Instr>, Vec<Instr>)>,
        else_body: Option<Vec<Instr>>,
    },
    While {
        cond: Vec<Instr>,
        body: Vec<Instr>,
    },
    ForRange {
        start: Vec<Instr>,
        end: Vec<Instr>,
        var: String,
        body: Vec<Instr>,
    },
    ForEach {
        iter: Vec<Instr>,
        var: String,
        body: Vec<Instr>,
    },
    Return(Option<Vec<Instr>>),
    Use {
        path: String,
        namespace: Option<String>,
    },
}

pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    pub fn compile_program(&mut self, program: &Program) -> Result<Vec<Instr>, RuntimeError> {
        self.compile_stmts(&program.statements)
    }

    fn compile_stmts(&mut self, stmts: &[Stmt]) -> Result<Vec<Instr>, RuntimeError> {
        let mut code = Vec::new();
        for stmt in stmts {
            self.compile_stmt(stmt, &mut code)?;
        }
        Ok(code)
    }

    fn compile_scoped_stmts(&mut self, stmts: &[Stmt]) -> Result<Vec<Instr>, RuntimeError> {
        self.compile_stmts(stmts)
    }

    fn compile_stmt(&mut self, stmt: &Stmt, code: &mut Vec<Instr>) -> Result<(), RuntimeError> {
        match stmt {
            Stmt::Let {
                name,
                annotation,
                value,
            } => {
                self.compile_expr(value, code)?;
                if let Some(ann) = annotation {
                    code.push(Instr::Coerce(*ann));
                }
                code.push(Instr::DefineVar(name.clone()));
            }
            Stmt::Assign { target, op, value } => match target {
                Expr::Var(name) => {
                    self.compile_expr(value, code)?;
                    code.push(Instr::AssignVar {
                        name: name.clone(),
                        op: *op,
                    });
                }
                _ => {
                    return Err(RuntimeError::InvalidOperation(
                        "assignment target must be a variable".to_string(),
                    ))
                }
            },
            Stmt::If {
                cond,
                then_block,
                elif_blocks,
                else_block,
            } => {
                let then_code = self.compile_scoped_stmts(then_block)?;
                let mut compiled_elifs = Vec::with_capacity(elif_blocks.len());
                for (elif_cond, block) in elif_blocks {
                    compiled_elifs.push((
                        self.compile_expr_chunk(elif_cond)?,
                        self.compile_scoped_stmts(block)?,
                    ));
                }
                let else_code = match else_block {
                    Some(block) => Some(self.compile_scoped_stmts(block)?),
                    None => None,
                };
                code.push(Instr::If {
                    cond: self.compile_expr_chunk(cond)?,
                    then_body: then_code,
                    elifs: compiled_elifs,
                    else_body: else_code,
                });
            }
            Stmt::While { cond, body } => {
                code.push(Instr::While {
                    cond: self.compile_expr_chunk(cond)?,
                    body: self.compile_scoped_stmts(body)?,
                });
            }
            Stmt::ForRange {
                var,
                start,
                end,
                body,
            } => {
                code.push(Instr::ForRange {
                    start: self.compile_expr_chunk(start)?,
                    end: self.compile_expr_chunk(end)?,
                    var: var.clone(),
                    body: self.compile_scoped_stmts(body)?,
                });
            }
            Stmt::ForEach { var, iter, body } => {
                code.push(Instr::ForEach {
                    iter: self.compile_expr_chunk(iter)?,
                    var: var.clone(),
                    body: self.compile_scoped_stmts(body)?,
                });
            }
            Stmt::FnDef {
                name,
                params,
                ret_type,
                body,
            } => {
                let function = CompiledFunction {
                    params: params.to_vec(),
                    ret_type: *ret_type,
                    code: self.compile_stmts(body)?,
                };
                code.push(Instr::DefineFunction {
                    name: name.clone(),
                    function,
                });
            }
            Stmt::Return(expr) => {
                let compiled = match expr {
                    Some(expr) => Some(self.compile_expr_chunk(expr)?),
                    None => None,
                };
                code.push(Instr::Return(compiled));
            }
            Stmt::Use { path, namespace } => code.push(Instr::Use {
                path: path.clone(),
                namespace: namespace.clone(),
            }),
            Stmt::Expr(expr) => {
                self.compile_expr(expr, code)?;
                code.push(Instr::Pop);
            } // EXTENSION POINT: add new statement compilers here.
        }
        Ok(())
    }

    fn compile_expr_chunk(&mut self, expr: &Expr) -> Result<Vec<Instr>, RuntimeError> {
        let mut code = Vec::new();
        self.compile_expr(expr, &mut code)?;
        Ok(code)
    }

    fn compile_expr(&mut self, expr: &Expr, code: &mut Vec<Instr>) -> Result<(), RuntimeError> {
        match expr {
            Expr::Value(v) => code.push(Instr::Push(v.clone())),
            Expr::Var(name) => code.push(Instr::LoadVar(name.clone())),
            Expr::InterpolatedString(raw) => {
                code.push(Instr::Interpolate(parse_interpolated_parts(raw)));
            }
            Expr::List(items) => {
                for item in items.iter() {
                    self.compile_expr(item, code)?;
                }
                code.push(Instr::MakeList(items.len()));
            }
            Expr::Json(entries) => {
                let mut keys = Vec::with_capacity(entries.len());
                for (key, value) in entries {
                    keys.push(key.clone());
                    self.compile_expr(value, code)?;
                }
                code.push(Instr::MakeJson(keys));
            }
            Expr::Unary { op, expr } => {
                self.compile_expr(expr, code)?;
                code.push(Instr::Unary(*op));
            }
            Expr::Binary { left, op, right } => match op {
                BinaryOp::And => {
                    self.compile_expr(left, code)?;
                    self.compile_expr(right, code)?;
                    code.push(Instr::Binary(*op));
                }
                BinaryOp::Or => {
                    self.compile_expr(left, code)?;
                    self.compile_expr(right, code)?;
                    code.push(Instr::Binary(*op));
                }
                _ => {
                    self.compile_expr(left, code)?;
                    self.compile_expr(right, code)?;
                    code.push(Instr::Binary(*op));
                }
            },
            Expr::Call { callee, args } => {
                let name = match &**callee {
                    Expr::Var(name) => name.clone(),
                    _ => {
                        return Err(RuntimeError::InvalidOperation(
                            "call target must be an identifier".to_string(),
                        ))
                    }
                };
                for arg in args {
                    self.compile_expr(arg, code)?;
                }
                code.push(Instr::CallFunction {
                    name,
                    argc: args.len(),
                });
            }
            Expr::ModuleCall {
                module,
                function,
                args,
            } => {
                for arg in args {
                    self.compile_expr(arg, code)?;
                }
                code.push(Instr::CallModule {
                    module: module.clone(),
                    function: function.clone(),
                    argc: args.len(),
                });
            }
            Expr::StdModuleCall {
                module,
                function,
                args,
            } => {
                for arg in args {
                    self.compile_expr(arg, code)?;
                }
                code.push(Instr::CallStdModule {
                    module: module.clone(),
                    function: function.clone(),
                    argc: args.len(),
                });
            }
            Expr::Index { object, index } => {
                self.compile_expr(object, code)?;
                self.compile_expr(index, code)?;
                code.push(Instr::GetIndex);
            }
        }
        Ok(())
    }
}

pub type BuiltinFn = fn(Vec<Value>) -> Result<Value, StdlibError>;
pub type ModuleFn = fn(Vec<Value>) -> Result<Value, StdlibError>;





impl Stdlib {
    pub fn new() -> Self {
        Self {
            builtins: register_builtins(),
            modules: register_modules(),
        }
    }

    pub fn call_builtin(&self, name: &str, args: Vec<Value>) -> Result<Value, StdlibError> {
        let func = self
            .builtins
            .get(name)
            .ok_or_else(|| StdlibError::Message(format!("Unknown builtin: {name}")))?;
        func(args)
    }

    pub fn call_module(
        &self,
        module: &str,
        func: &str,
        args: Vec<Value>,
    ) -> Result<Value, StdlibError> {
        let module_map = self
            .modules
            .get(module)
            .ok_or_else(|| StdlibError::Message(format!("Unknown module: {module}")))?;
        let f = module_map.get(func).ok_or_else(|| {
            StdlibError::Message(format!("Unknown module function: {module}.{func}"))
        })?;
        f(args)
    }
}

fn builtin_out(args: Vec<Value>) -> Result<Value, StdlibError> {
    if let Some(v) = args.first() {
        print!("{}", v.to_pretty_string());
    } else {
        print!("");
    }
    Ok(Value::Null)
}

fn builtin_outln(args: Vec<Value>) -> Result<Value, StdlibError> {
    if let Some(v) = args.first() {
        println!("{}", v.to_pretty_string());
    } else {
        println!();
    }
    Ok(Value::Null)
}

fn builtin_run(args: Vec<Value>) -> Result<Value, StdlibError> {
    let cmd = match args.first() {
        Some(Value::Str(s)) => s,
        _ => {
            return Err(StdlibError::Message(
                "run(cmd) expects a single string argument".to_string(),
            ))
        }
    };

    let output = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .arg("/C")
            .arg(cmd.as_ref())
            .output()
    } else {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd.as_ref())
            .output()
    }
    .map_err(|e| StdlibError::Message(format!("run failed: {e}")))?;

    Ok(Value::Str(Arc::from(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    )))
}

fn register_builtins() -> HashMap<String, BuiltinFn> {
    let mut map: HashMap<String, BuiltinFn> = HashMap::new();
    map.insert("out".to_string(), builtin_out as BuiltinFn);
    map.insert("outln".to_string(), builtin_outln as BuiltinFn);
    map.insert("log".to_string(), builtin_outln as BuiltinFn);
    map.insert("run".to_string(), builtin_run as BuiltinFn);
    map
}

fn register_modules() -> HashMap<String, HashMap<String, ModuleFn>> {
    let mut modules: HashMap<String, HashMap<String, ModuleFn>> = HashMap::new();
    modules.insert("net".to_string(), net::register());
    modules.insert("udp".to_string(), udp::register());
    modules
}

mod net {
    use super::{ModuleFn, StdlibError};
    use ast::Value;
    use std::collections::HashMap;
    use std::net::{TcpStream, ToSocketAddrs};
    use std::sync::Arc;
    use std::time::Duration;

    pub fn register() -> HashMap<String, ModuleFn> {
        let mut map: HashMap<String, ModuleFn> = HashMap::new();
        map.insert("ping".to_string(), ping as ModuleFn);
        map.insert("port_open".to_string(), port_open as ModuleFn);
        map.insert("resolve".to_string(), resolve as ModuleFn);
        map
    }

    fn ping(args: Vec<Value>) -> Result<Value, StdlibError> {
        let host = match args.first() {
            Some(Value::Str(s)) => s,
            _ => {
                return Err(StdlibError::Message(
                    "net.ping(host) expects string host".to_string(),
                ))
            }
        };

        let output = if cfg!(target_os = "windows") {
            std::process::Command::new("ping")
                .arg("-n")
                .arg("1")
                .arg(host.as_ref())
                .output()
        } else {
            std::process::Command::new("ping")
                .arg("-c")
                .arg("1")
                .arg(host.as_ref())
                .output()
        }
        .map_err(|e| StdlibError::Message(format!("ping failed: {e}")))?;

        Ok(Value::Bool(output.status.success()))
    }

    fn port_open(args: Vec<Value>) -> Result<Value, StdlibError> {
        if args.len() != 2 {
            return Err(StdlibError::Message(
                "net.port_open(host, port) expects 2 arguments".to_string(),
            ));
        }
        let host = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => {
                return Err(StdlibError::Message(
                    "net.port_open host must be string".to_string(),
                ))
            }
        };
        let port = match &args[1] {
            Value::Int(p) => *p as u16,
            _ => {
                return Err(StdlibError::Message(
                    "net.port_open port must be int".to_string(),
                ))
            }
        };

        let addr = format!("{host}:{port}");
        let timeout = Duration::from_secs(2);
        let result = addr
            .to_socket_addrs()
            .map_err(|e| StdlibError::Message(format!("resolve failed: {e}")))?
            .any(|socket_addr| TcpStream::connect_timeout(&socket_addr, timeout).is_ok());

        Ok(Value::Bool(result))
    }

    fn resolve(args: Vec<Value>) -> Result<Value, StdlibError> {
        let host = match args.first() {
            Some(Value::Str(s)) => s,
            _ => {
                return Err(StdlibError::Message(
                    "net.resolve(hostname) expects string".to_string(),
                ))
            }
        };

        let addr = format!("{host}:0");
        let first = addr
            .to_socket_addrs()
            .ok()
            .and_then(|mut iter| iter.next())
            .map(|a| a.ip().to_string());

        Ok(match first {
            Some(ip) => Value::Str(Arc::from(ip)),
            None => Value::Null,
        })
    }
}

mod udp {
    use super::{ModuleFn, StdlibError};
    use ast::Value;
    use std::collections::HashMap;

    pub fn register() -> HashMap<String, ModuleFn> {
        let mut map: HashMap<String, ModuleFn> = HashMap::new();
        map.insert("test".to_string(), test as ModuleFn);
        map
    }

    fn test(_args: Vec<Value>) -> Result<Value, StdlibError> {
        panic!("INTERPRETER UDP");
        Ok(Value::Bool(true))
    }
}

pub struct Interpreter {
    scopes: Vec<ScopeFrame>,
    functions: HashMap<String, CompiledFunction>,
    user_modules: HashMap<String, ModuleState>,
    import_cache: HashMap<PathBuf, CachedImport>,
    stdlib: Stdlib,
    base_dir: PathBuf,
}

impl Interpreter {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            scopes: vec![ScopeFrame::new()],
            functions: HashMap::new(),
            user_modules: HashMap::new(),
            import_cache: HashMap::new(),
            stdlib: Stdlib::new(),
            base_dir: base_dir.into(),
        }
    }

    pub fn execute_program(&mut self, program: &Program) -> Result<(), RuntimeError> {
        let mut compiler = Compiler::new();
        let code = compiler.compile_program(program)?;
        match self.run_code(&code)? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Return(_) => Err(RuntimeError::InvalidOperation(
                "return outside of function".to_string(),
            )),
        }
    }

    pub fn execute_program_from_bytecode(&mut self, code: &[Instr]) -> Result<(), RuntimeError> {
        match self.run_code(code)? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Return(_) => Err(RuntimeError::InvalidOperation(
                "return outside of function".to_string(),
            )),
        }
    }

    fn run_code(&mut self, code: &[Instr]) -> Result<ControlFlow, RuntimeError> {
        let mut stack: Vec<Value> = Vec::new();
        for instr in code {
            match instr {
                Instr::Push(v) => stack.push(v.clone()),
                Instr::LoadVar(name) => stack.push(self.get_var(name)?),
                Instr::Unary(op) => {
                    let value = pop_value(&mut stack, "unary operation")?;
                    stack.push(self.apply_unary(*op, &value)?);
                }
                Instr::Binary(op) => {
                    let right = pop_value(&mut stack, "binary operation")?;
                    let left = pop_value(&mut stack, "binary operation")?;
                    stack.push(self.apply_binop(&left, *op, &right)?);
                }
                Instr::MakeList(count) => {
                    let mut items = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        items.push(pop_value(&mut stack, "list literal")?);
                    }
                    items.reverse();
                    stack.push(Value::List(Arc::from(items)));
                }
                Instr::MakeJson(keys) => {
                    let mut out = BTreeMap::new();
                    for key in keys.iter().rev() {
                        let value = pop_value(&mut stack, "json literal")?;
                        out.insert(key.clone(), value);
                    }
                    stack.push(Value::Json(Arc::new(out)));
                }
                Instr::Interpolate(parts) => {
                    let mut out = String::new();
                    for part in parts {
                        match part {
                            InterpPart::Text(text) => out.push_str(text),
                            InterpPart::Var(name) => {
                                out.push_str(&self.get_var(name)?.to_pretty_string());
                            }
                        }
                    }
                    stack.push(Value::Str(Arc::from(out)));
                }
                Instr::CallFunction { name, argc } => {
                    let args = pop_args(&mut stack, *argc, "function call")?;
                    let value = self.call_function(name, args)?;
                    stack.push(value);
                }
                Instr::CallModule {
                    module,
                    function,
                    argc,
                } => {
                    let args = pop_args(&mut stack, *argc, "module call")?;
                    let value = self.call_module(module, function, args)?;
                    stack.push(value);
                }
                Instr::CallStdModule {
                    module,
                    function,
                    argc,
                } => {
                    let args = pop_args(&mut stack, *argc, "std module call")?;
                    let value = self.call_std_module(module, function, args)?;
                    stack.push(value);
                }
                Instr::GetIndex => {
                    let idx = pop_value(&mut stack, "indexing")?;
                    let obj = pop_value(&mut stack, "indexing")?;
                    stack.push(self.eval_index(obj, idx)?);
                }
                Instr::Coerce(target) => {
                    let value = pop_value(&mut stack, "type coercion")?;
                    let coerced = value.coerce_to(*target).ok_or_else(|| {
                        RuntimeError::TypeError(format!(
                            "cannot coerce '{}' to {:?}",
                            value.type_name(),
                            target
                        ))
                    })?;
                    stack.push(coerced);
                }
                Instr::Pop => {
                    pop_value(&mut stack, "pop")?;
                }
                Instr::DefineVar(name) => {
                    let value = pop_value(&mut stack, "variable definition")?;
                    self.current_scope_mut().define(name.clone(), value);
                }
                Instr::AssignVar { name, op } => {
                    let rhs = pop_value(&mut stack, "assignment")?;
                    let next = match op {
                        AssignOp::Assign => rhs,
                        AssignOp::AddAssign => {
                            let current = self.get_var(name)?;
                            self.apply_binop(&current, BinaryOp::Add, &rhs)?
                        }
                        AssignOp::SubAssign => {
                            let current = self.get_var(name)?;
                            self.apply_binop(&current, BinaryOp::Sub, &rhs)?
                        }
                        AssignOp::MulAssign => {
                            let current = self.get_var(name)?;
                            self.apply_binop(&current, BinaryOp::Mul, &rhs)?
                        }
                        AssignOp::DivAssign => {
                            let current = self.get_var(name)?;
                            self.apply_binop(&current, BinaryOp::Div, &rhs)?
                        }
                    };
                    self.set_var(name, next)?;
                }
                Instr::DefineFunction { name, function } => {
                    self.functions.insert(name.clone(), function.clone());
                }
                Instr::If {
                    cond,
                    then_body,
                    elifs,
                    else_body,
                } => {
                    if self.run_expr_chunk(cond)?.is_truthy() {
                        match self.run_block(then_body)? {
                            ControlFlow::Continue => {}
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        }
                    } else {
                        let mut matched = false;
                        for (elif_cond, elif_body) in elifs {
                            if self.run_expr_chunk(elif_cond)?.is_truthy() {
                                matched = true;
                                match self.run_block(elif_body)? {
                                    ControlFlow::Continue => {}
                                    ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                }
                                break;
                            }
                        }
                        if !matched {
                            if let Some(block) = else_body {
                                match self.run_block(block)? {
                                    ControlFlow::Continue => {}
                                    ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                }
                            }
                        }
                    }
                }
                Instr::While { cond, body } => loop {
                    if !self.run_expr_chunk(cond)?.is_truthy() {
                        break;
                    }
                    match self.run_block(body)? {
                        ControlFlow::Continue => {}
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                    }
                },
                Instr::ForRange {
                    start,
                    end,
                    var,
                    body,
                } => {
                    let start_value = self.run_expr_chunk(start)?;
                    let end_value = self.run_expr_chunk(end)?;
                    let (start_i, end_i) = match (start_value, end_value) {
                        (Value::Int(s), Value::Int(e)) => (s, e),
                        _ => {
                            return Err(RuntimeError::TypeError(
                                "for range requires int bounds".to_string(),
                            ))
                        }
                    };
                    for i in start_i..end_i {
                        self.current_scope_mut().define(var.clone(), Value::Int(i));
                        match self.run_block(body)? {
                            ControlFlow::Continue => {}
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        }
                    }
                }
                Instr::ForEach { iter, var, body } => {
                    let iterable = self.run_expr_chunk(iter)?;
                    let items = match iterable {
                        Value::List(items) => items,
                        _ => {
                            return Err(RuntimeError::TypeError(
                                "for each requires list iterable".to_string(),
                            ))
                        }
                    };
                    for item in items.iter().cloned() {
                        self.current_scope_mut().define(var.clone(), item);
                        match self.run_block(body)? {
                            ControlFlow::Continue => {}
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        }
                    }
                }
                Instr::Return(expr) => {
                    let value = if let Some(expr) = expr {
                        self.run_expr_chunk(expr)?
                    } else {
                        Value::Null
                    };
                    return Ok(ControlFlow::Return(value));
                }
                Instr::Use { path, namespace } => {
                    self.run_use(path, namespace.as_deref())?;
                }
            }
        }
        Ok(ControlFlow::Continue)
    }

    fn run_expr_chunk(&mut self, code: &[Instr]) -> Result<Value, RuntimeError> {
        let mut stack: Vec<Value> = Vec::new();
        for instr in code {
            match instr {
                Instr::Push(v) => stack.push(v.clone()),
                Instr::LoadVar(name) => stack.push(self.get_var(name)?),
                Instr::Unary(op) => {
                    let value = pop_value(&mut stack, "unary expression")?;
                    stack.push(self.apply_unary(*op, &value)?);
                }
                Instr::Binary(op) => {
                    let right = pop_value(&mut stack, "binary expression")?;
                    let left = pop_value(&mut stack, "binary expression")?;
                    stack.push(self.apply_binop(&left, *op, &right)?);
                }
                Instr::MakeList(count) => {
                    let mut items = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        items.push(pop_value(&mut stack, "list expression")?);
                    }
                    items.reverse();
                    stack.push(Value::List(Arc::from(items)));
                }
                Instr::MakeJson(keys) => {
                    let mut out = BTreeMap::new();
                    for key in keys.iter().rev() {
                        let value = pop_value(&mut stack, "json expression")?;
                        out.insert(key.clone(), value);
                    }
                    stack.push(Value::Json(Arc::new(out)));
                }
                Instr::Interpolate(parts) => {
                    let mut out = String::new();
                    for part in parts {
                        match part {
                            InterpPart::Text(text) => out.push_str(text),
                            InterpPart::Var(name) => {
                                out.push_str(&self.get_var(name)?.to_pretty_string());
                            }
                        }
                    }
                    stack.push(Value::Str(Arc::from(out)));
                }
                Instr::CallFunction { name, argc } => {
                    let args = pop_args(&mut stack, *argc, "function expression")?;
                    let value = self.call_function(name, args)?;
                    stack.push(value);
                }
                Instr::CallModule {
                    module,
                    function,
                    argc,
                } => {
                    let args = pop_args(&mut stack, *argc, "module expression")?;
                    let value = self.call_module(module, function, args)?;
                    stack.push(value);
                }
                Instr::CallStdModule {
                    module,
                    function,
                    argc,
                } => {
                    let args = pop_args(&mut stack, *argc, "std module expression")?;
                    let value = self.call_std_module(module, function, args)?;
                    stack.push(value);
                }
                Instr::GetIndex => {
                    let idx = pop_value(&mut stack, "indexing")?;
                    let obj = pop_value(&mut stack, "indexing")?;
                    stack.push(self.eval_index(obj, idx)?);
                }
                Instr::Coerce(target) => {
                    let value = pop_value(&mut stack, "type coercion")?;
                    let coerced = value.coerce_to(*target).ok_or_else(|| {
                        RuntimeError::TypeError(format!(
                            "cannot coerce '{}' to {:?}",
                            value.type_name(),
                            target
                        ))
                    })?;
                    stack.push(coerced);
                }
                Instr::Pop
                | Instr::DefineVar(_)
                | Instr::AssignVar { .. }
                | Instr::DefineFunction { .. }
                | Instr::If { .. }
                | Instr::While { .. }
                | Instr::ForRange { .. }
                | Instr::ForEach { .. }
                | Instr::Return(_)
                | Instr::Use { .. } => {
                    return Err(RuntimeError::InvalidOperation(
                        "statement instruction used in expression context".to_string(),
                    ))
                }
            }
        }
        stack.pop().ok_or_else(|| {
            RuntimeError::InvalidOperation("expression did not produce a value".to_string())
        })
    }

    fn run_block(&mut self, code: &[Instr]) -> Result<ControlFlow, RuntimeError> {
        self.scopes.push(ScopeFrame::new());
        let result = self.run_code(code);
        self.scopes.pop();
        result
    }

    fn run_use(&mut self, path: &str, namespace: Option<&str>) -> Result<(), RuntimeError> {
        let full = self.base_dir.join(path);
        let module_name = namespace
            .map(|s| s.to_string())
            .or_else(|| module_name_from_path(path))
            .ok_or_else(|| {
                RuntimeError::ImportError(format!(
                    "cannot derive module name from import path: {path}"
                ))
            })?;

        let program = self.load_import_program(&full)?;

        let mut child = Interpreter::new(parent_of(&full));
        child.scopes = self.scopes.clone();
        child.functions = self.functions.clone();
        child.user_modules = self.user_modules.clone();
        child.import_cache = self.import_cache.clone();
        child.execute_program(&program)?;

        let module_state = child.into_module_state();
        self.scopes = module_state.scopes.clone();
        self.functions = module_state.functions.clone();
        self.user_modules.insert(module_name, module_state);
        Ok(())
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
        if let Some(func) = self.stdlib.builtins.get(name) {
            return func(args).map_err(|e| RuntimeError::InvalidOperation(e.to_string()));
        }

        let f = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::UndefinedVariable(name.to_string()))?;

        self.call_compiled_function(name, f, args)
    }

    fn call_compiled_function(
        &mut self,
        name: &str,
        f: CompiledFunction,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if args.len() != f.params.len() {
            return Err(RuntimeError::InvalidOperation(format!(
                "function '{name}' expects {} args, got {}",
                f.params.len(),
                args.len()
            )));
        }

        self.scopes.push(ScopeFrame::new());
        for (param, arg) in f.params.iter().zip(args.into_iter()) {
            let val = if let Some(ann) = param.annotation {
                arg.coerce_to(ann).ok_or_else(|| {
                    RuntimeError::TypeError(format!(
                        "cannot coerce arg '{}' to {:?}",
                        arg.type_name(),
                        ann
                    ))
                })?
            } else {
                arg
            };
            self.current_scope_mut().define(param.name.clone(), val);
        }

        let run_result = self.run_code(&f.code);
        self.scopes.pop();

        let result = match run_result? {
            ControlFlow::Continue => Value::Null,
            ControlFlow::Return(v) => v,
        };

        if let Some(ann) = f.ret_type {
            result.coerce_to(ann).ok_or_else(|| {
                RuntimeError::TypeError(format!(
                    "cannot coerce return '{}' to {:?}",
                    result.type_name(),
                    ann
                ))
            })
        } else {
            Ok(result)
        }
    }

    fn load_import_program(&mut self, full: &Path) -> Result<Program, RuntimeError> {
        let key = fs::canonicalize(full).unwrap_or_else(|_| full.to_path_buf());
        let modified = fs::metadata(full).and_then(|m| m.modified()).ok();

        if let Some(cached) = self.import_cache.get(&key) {
            if cached.modified == modified {
                return Ok(cached.program.clone());
            }
        }

        let content = fs::read_to_string(full)
            .map_err(|e| RuntimeError::ImportError(format!("{}: {e}", full.display())))?;
        let tokens =
            lexer::tokenize(&content).map_err(|e| RuntimeError::ParseError(e.to_string()))?;
        let program = parser::parse(tokens).map_err(|e| RuntimeError::ParseError(e.to_string()))?;
        self.import_cache.insert(
            key,
            CachedImport {
                modified,
                program: program.clone(),
            },
        );
        Ok(program)
    }

    fn call_module(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if let Some(state) = self.user_modules.get(module).cloned() {
            return self.call_user_module_function(module, function, args, state);
        }

        self.stdlib
            .call_module(module, function, args)
            .map_err(|e| RuntimeError::ModuleError(e.to_string()))
    }

    fn call_std_module(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        self.stdlib
            .call_module(module, function, args)
            .map_err(|e| RuntimeError::ModuleError(e.to_string()))
    }

    fn call_user_module_function(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
        state: ModuleState,
    ) -> Result<Value, RuntimeError> {
        let mut child = Interpreter::from_module_state(state);
        let result = child.call_function(function, args);
        if result.is_ok() {
            self.user_modules
                .insert(module.to_string(), child.into_module_state());
        }
        result
    }

    fn current_scope_mut(&mut self) -> &mut ScopeFrame {
        self.scopes.last_mut().expect("scope stack is non-empty")
    }

    fn get_var(&self, name: &str) -> Result<Value, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Ok(v.clone());
            }
        }
        Err(RuntimeError::UndefinedVariable(name.to_string()))
    }

    fn set_var(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        for idx in (0..self.scopes.len()).rev() {
            if self.scopes[idx].names.contains_key(name) {
                self.scopes[idx].set(name, value);
                return Ok(());
            }
        }
        Err(RuntimeError::UndefinedVariable(name.to_string()))
    }

    fn eval_index(&self, obj: Value, idx: Value) -> Result<Value, RuntimeError> {
        match (obj, idx) {
            (Value::List(items), Value::Int(i)) => {
                items.get(i as usize).cloned().ok_or_else(|| {
                    RuntimeError::InvalidOperation("list index out of bounds".to_string())
                })
            }
            (Value::Json(map), Value::Str(key)) => {
                Ok(map.get(key.as_ref()).cloned().unwrap_or(Value::Null))
            }
            _ => Err(RuntimeError::TypeError(
                "indexing requires list[int] or json[str]".to_string(),
            )),
        }
    }

    fn apply_unary(&self, op: UnaryOp, value: &Value) -> Result<Value, RuntimeError> {
        match op {
            UnaryOp::Not => Ok(Value::Bool(!value.is_truthy())),
            UnaryOp::Neg => match value {
                Value::Int(v) => Ok(Value::Int(-v)),
                Value::Float(v) => Ok(Value::Float(-v)),
                _ => Err(RuntimeError::TypeError(
                    "unary '-' expects int or float".to_string(),
                )),
            },
        }
    }

    fn apply_binop(
        &self,
        left: &Value,
        op: BinaryOp,
        right: &Value,
    ) -> Result<Value, RuntimeError> {
        match op {
            BinaryOp::Add => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                (Value::Str(a), Value::Str(b)) => {
                    let mut s = String::with_capacity(a.len() + b.len());
                    s.push_str(a);
                    s.push_str(b);
                    Ok(Value::Str(Arc::from(s)))
                }
                _ => Err(RuntimeError::TypeError("invalid '+' operands".to_string())),
            },
            BinaryOp::Sub => num_binop(left, right, |a, b| a - b, |a, b| a - b),
            BinaryOp::Mul => num_binop(left, right, |a, b| a * b, |a, b| a * b),
            BinaryOp::Div => num_binop(left, right, |a, b| a / b, |a, b| a / b),
            BinaryOp::FloorDiv => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
                _ => Err(RuntimeError::TypeError(
                    "'//' expects int operands".to_string(),
                )),
            },
            BinaryOp::Mod => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
                _ => Err(RuntimeError::TypeError(
                    "'%' expects int operands".to_string(),
                )),
            },
            BinaryOp::Pow => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.pow(*b as u32))),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).powf(*b))),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.powf(*b as f64))),
                _ => Err(RuntimeError::TypeError(
                    "'**' expects numeric operands".to_string(),
                )),
            },
            BinaryOp::Eq => Ok(Value::Bool(value_eq(left, right))),
            BinaryOp::Ne => Ok(Value::Bool(!value_eq(left, right))),
            BinaryOp::Lt => cmp_binop(left, right, |a, b| a < b),
            BinaryOp::Gt => cmp_binop(left, right, |a, b| a > b),
            BinaryOp::Le => cmp_binop(left, right, |a, b| a <= b),
            BinaryOp::Ge => cmp_binop(left, right, |a, b| a >= b),
            BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
        }
    }

    fn into_module_state(self) -> ModuleState {
        ModuleState {
            scopes: self.scopes,
            functions: self.functions,
            user_modules: self.user_modules,
            // import cache is runtime-only and shouldn't leak across module snapshots
            base_dir: self.base_dir,
        }
    }

    fn from_module_state(state: ModuleState) -> Self {
        Self {
            scopes: state.scopes,
            functions: state.functions,
            user_modules: state.user_modules,
            import_cache: HashMap::new(),
            stdlib: Stdlib::new(),
            base_dir: state.base_dir,
        }
    }
}

enum ControlFlow {
    Continue,
    Return(Value),
}

fn pop_value(stack: &mut Vec<Value>, context: &str) -> Result<Value, RuntimeError> {
    stack
        .pop()
        .ok_or_else(|| RuntimeError::InvalidOperation(format!("stack underflow during {context}")))
}

fn pop_args(
    stack: &mut Vec<Value>,
    argc: usize,
    context: &str,
) -> Result<Vec<Value>, RuntimeError> {
    if stack.len() < argc {
        return Err(RuntimeError::InvalidOperation(format!(
            "stack underflow during {context}"
        )));
    }
    let mut args = Vec::with_capacity(argc);
    for _ in 0..argc {
        args.push(stack.pop().expect("length checked above"));
    }
    args.reverse();
    Ok(args)
}

fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) => *x as f64 == *y,
        (Value::Float(x), Value::Int(y)) => *x == *y as f64,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn num_binop(
    left: &Value,
    right: &Value,
    int_op: fn(i64, i64) -> i64,
    float_op: fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a, *b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(*a as f64, *b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(*a, *b as f64))),
        _ => Err(RuntimeError::TypeError(
            "numeric operands required".to_string(),
        )),
    }
}

fn cmp_binop(left: &Value, right: &Value, op: fn(f64, f64) -> bool) -> Result<Value, RuntimeError> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(op(*a as f64, *b as f64))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(op(*a, *b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool(op(*a as f64, *b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(op(*a, *b as f64))),
        _ => Err(RuntimeError::TypeError(
            "comparison requires numeric operands".to_string(),
        )),
    }
}

fn parent_of(path: &Path) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn module_name_from_path(path: &str) -> Option<String> {
    let stem = Path::new(path).file_stem()?.to_str()?;
    let mut chars = stem.chars();
    let first = chars.next()?;
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return None;
    }
    if chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        Some(stem.to_string())
    } else {
        None
    }
}

fn parse_interpolated_parts(raw: &str) -> Vec<InterpPart> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if !buf.is_empty() {
                parts.push(InterpPart::Text(std::mem::take(&mut buf)));
            }
            let mut name = String::new();
            while let Some(next) = chars.next() {
                if next == '}' {
                    break;
                }
                name.push(next);
            }
            parts.push(InterpPart::Var(name.trim().to_string()));
        } else {
            buf.push(ch);
        }
    }

    if !buf.is_empty() {
        parts.push(InterpPart::Text(buf));
    }

    parts
}
/// Сохранить скомпилированный bytecode в файл .vcbc
pub fn save_bytecode(code: &[Instr], path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(code)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Загрузить bytecode из файла .vcbc
pub fn load_bytecode(path: &Path) -> Result<Vec<Instr>, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    let code = serde_json::from_str(&json)?;
    Ok(code)
}
