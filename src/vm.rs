use ast::{AssignOp, BinaryOp, Program, UnaryOp, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::bytecode::{Bytecode, CompiledFunction, Instr, InterpPart};

#[derive(Debug, thiserror::Error)]
pub enum VmError {
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

#[derive(Clone)]
struct ScopeFrame {
    names: HashMap<String, usize>,
    values: Vec<Value>,
}

impl ScopeFrame {
    fn new() -> Self {
        Self { names: HashMap::new(), values: Vec::new() }
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

#[derive(Clone)]
struct ModuleState {
    scopes: Vec<ScopeFrame>,
    functions: HashMap<String, CompiledFunction>,
    user_modules: HashMap<String, ModuleState>,
    base_dir: PathBuf,
}

pub struct Vm {
    scopes: Vec<ScopeFrame>,
    functions: HashMap<String, CompiledFunction>,
    user_modules: HashMap<String, ModuleState>,
    import_cache: HashMap<PathBuf, CachedImport>,
    stdlib: stdlib::Stdlib,
    base_dir: PathBuf,
}

impl Vm {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            scopes: vec![ScopeFrame::new()],
            functions: HashMap::new(),
            user_modules: HashMap::new(),
            import_cache: HashMap::new(),
            stdlib: stdlib::Stdlib::new(),
            base_dir: base_dir.into(),
        }
    }

    pub fn run_program(&mut self, program: &Program) -> Result<(), VmError> {
        let mut t = crate::bytecode_translator::BytecodeTranslator::new();
        let code = t.translate_program(program).map_err(|e| VmError::InvalidOperation(e.to_string()))?;
        self.run_bytecode(&code)
    }

    pub fn run_bytecode(&mut self, code: &Bytecode) -> Result<(), VmError> {
        match self.run_code(&code.code)? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Return(_) => Err(VmError::InvalidOperation("return outside of function".to_string())),
        }
    }

    fn run_code(&mut self, code: &[Instr]) -> Result<ControlFlow, VmError> {
        let mut stack: Vec<Value> = Vec::new();
        for instr in code {
            match instr {
                Instr::Const(v) | Instr::Push(v) => stack.push(v.clone()),
                Instr::LoadLocal(name) | Instr::LoadVar(name) => stack.push(self.get_var(name)?),
                Instr::Unary(op) => {
                    let value = pop_value(&mut stack, "unary operation")?;
                    stack.push(self.apply_unary(*op, &value)?);
                }
                Instr::Binary(op) => {
                    let right = pop_value(&mut stack, "binary operation")?;
                    let left = pop_value(&mut stack, "binary operation")?;
                    stack.push(self.apply_binop(&left, *op, &right)?);
                }
                Instr::BuildList(count) | Instr::MakeList(count) => {
                    let mut items = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        items.push(pop_value(&mut stack, "list literal")?);
                    }
                    items.reverse();
                    stack.push(Value::List(Arc::from(items)));
                }
                Instr::BuildMap(keys) | Instr::MakeJson(keys) => {
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
                            InterpPart::Var(name) => out.push_str(&self.get_var(name)?.to_pretty_string()),
                        }
                    }
                    stack.push(Value::Str(Arc::from(out)));
                }
                Instr::CallLocal { name, argc } | Instr::CallFunction { name, argc } => {
                    let args = pop_args(&mut stack, *argc, "function call")?;
                    stack.push(self.call_function(name, args)?);
                }
                Instr::CallModule { module, function, argc } => {
                    let args = pop_args(&mut stack, *argc, "module call")?;
                    stack.push(self.call_module(module, function, args)?);
                }
                Instr::CallStd { module, function, argc } | Instr::CallStdModule { module, function, argc } => {
                    let args = pop_args(&mut stack, *argc, "std module call")?;
                    stack.push(self.call_std_module(module, function, args)?);
                }
                Instr::IndexGet | Instr::GetIndex => {
                    let idx = pop_value(&mut stack, "indexing")?;
                    let obj = pop_value(&mut stack, "indexing")?;
                    stack.push(self.eval_index(obj, idx)?);
                }
                Instr::Cast(target) | Instr::Coerce(target) => {
                    let value = pop_value(&mut stack, "type coercion")?;
                    let coerced = value.coerce_to(*target).ok_or_else(|| VmError::TypeError(format!(
                        "cannot coerce '{}' to {:?}",
                        value.type_name(),
                        target
                    )))?;
                    stack.push(coerced);
                }
                Instr::Pop => { let _ = pop_value(&mut stack, "pop")?; }
                Instr::StoreLocal(name) | Instr::DefineVar(name) => {
                    let value = pop_value(&mut stack, "variable definition")?;
                    self.current_scope_mut().define(name.clone(), value);
                }
                Instr::UpdateLocal { name, op } | Instr::AssignVar { name, op } => {
                    let rhs = pop_value(&mut stack, "assignment")?;
                    let next = match op {
                        AssignOp::Assign => rhs,
                        AssignOp::AddAssign => self.apply_binop(&self.get_var(name)?, BinaryOp::Add, &rhs)?,
                        AssignOp::SubAssign => self.apply_binop(&self.get_var(name)?, BinaryOp::Sub, &rhs)?,
                        AssignOp::MulAssign => self.apply_binop(&self.get_var(name)?, BinaryOp::Mul, &rhs)?,
                        AssignOp::DivAssign => self.apply_binop(&self.get_var(name)?, BinaryOp::Div, &rhs)?,
                    };
                    self.set_var(name, next)?;
                }
                Instr::DefineFunction { name, function } => {
                    self.functions.insert(name.clone(), function.clone());
                }
                Instr::If { cond, then_body, elifs, else_body } => {
                    if self.run_expr_chunk(&cond.code)?.is_truthy() {
                        match self.run_block(&then_body.code)? { ControlFlow::Continue => {}, ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)) }
                    } else {
                        let mut matched = false;
                        for (elif_cond, elif_body) in elifs {
                            if self.run_expr_chunk(&elif_cond.code)?.is_truthy() {
                                matched = true;
                                match self.run_block(&elif_body.code)? { ControlFlow::Continue => {}, ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)) }
                                break;
                            }
                        }
                        if !matched {
                            if let Some(block) = else_body {
                                match self.run_block(&block.code)? { ControlFlow::Continue => {}, ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)) }
                            }
                        }
                    }
                }
                Instr::While { cond, body } => loop {
                    if !self.run_expr_chunk(&cond.code)?.is_truthy() { break; }
                    match self.run_block(&body.code)? { ControlFlow::Continue => {}, ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)) }
                },
                Instr::ForRange { start, end, var, body } => {
                    let start_value = self.run_expr_chunk(&start.code)?;
                    let end_value = self.run_expr_chunk(&end.code)?;
                    let (start_i, end_i) = match (start_value, end_value) {
                        (Value::Int(s), Value::Int(e)) => (s, e),
                        _ => return Err(VmError::TypeError("for range requires int bounds".to_string())),
                    };
                    for i in start_i..end_i {
                        self.current_scope_mut().define(var.clone(), Value::Int(i));
                        match self.run_block(&body.code)? { ControlFlow::Continue => {}, ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)) }
                    }
                }
                Instr::ForEach { iter, var, body } => {
                    let iterable = self.run_expr_chunk(&iter.code)?;
                    let items = match iterable {
                        Value::List(items) => items,
                        _ => return Err(VmError::TypeError("for each requires list iterable".to_string())),
                    };
                    for item in items.iter().cloned() {
                        self.current_scope_mut().define(var.clone(), item);
                        match self.run_block(&body.code)? { ControlFlow::Continue => {}, ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)) }
                    }
                }
                Instr::Return(expr) => {
                    let value = if let Some(expr) = expr { self.run_expr_chunk(&expr.code)? } else { Value::Null };
                    return Ok(ControlFlow::Return(value));
                }
                Instr::Jump(_) | Instr::JumpIfFalse(_) | Instr::Loop(_) => {
                    return Err(VmError::InvalidOperation(
                        "jump opcodes are not active in this VM path yet".to_string(),
                    ))
                }
                Instr::Use { path, namespace } => self.run_use(path, namespace.as_deref())?,
            }
        }
        Ok(ControlFlow::Continue)
    }

    fn run_expr_chunk(&mut self, code: &[Instr]) -> Result<Value, VmError> {
        let mut stack: Vec<Value> = Vec::new();
        for instr in code {
            match instr {
                Instr::Const(v) | Instr::Push(v) => stack.push(v.clone()),
                Instr::LoadLocal(name) | Instr::LoadVar(name) => stack.push(self.get_var(name)?),
                Instr::Unary(op) => {
                    let value = pop_value(&mut stack, "unary expression")?;
                    stack.push(self.apply_unary(*op, &value)?);
                }
                Instr::Binary(op) => {
                    let right = pop_value(&mut stack, "binary expression")?;
                    let left = pop_value(&mut stack, "binary expression")?;
                    stack.push(self.apply_binop(&left, *op, &right)?);
                }
                Instr::BuildList(count) | Instr::MakeList(count) => {
                    let mut items = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        items.push(pop_value(&mut stack, "list expression")?);
                    }
                    items.reverse();
                    stack.push(Value::List(Arc::from(items)));
                }
                Instr::BuildMap(keys) | Instr::MakeJson(keys) => {
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
                            InterpPart::Var(name) => out.push_str(&self.get_var(name)?.to_pretty_string()),
                        }
                    }
                    stack.push(Value::Str(Arc::from(out)));
                }
                Instr::CallLocal { name, argc } | Instr::CallFunction { name, argc } => {
                    let args = pop_args(&mut stack, *argc, "function expression")?;
                    stack.push(self.call_function(name, args)?);
                }
                Instr::CallModule { module, function, argc } => {
                    let args = pop_args(&mut stack, *argc, "module expression")?;
                    stack.push(self.call_module(module, function, args)?);
                }
                Instr::CallStd { module, function, argc } | Instr::CallStdModule { module, function, argc } => {
                    let args = pop_args(&mut stack, *argc, "std module expression")?;
                    stack.push(self.call_std_module(module, function, args)?);
                }
                Instr::IndexGet | Instr::GetIndex => {
                    let idx = pop_value(&mut stack, "indexing")?;
                    let obj = pop_value(&mut stack, "indexing")?;
                    stack.push(self.eval_index(obj, idx)?);
                }
                Instr::Cast(target) | Instr::Coerce(target) => {
                    let value = pop_value(&mut stack, "type coercion")?;
                    let coerced = value.coerce_to(*target).ok_or_else(|| VmError::TypeError(format!(
                        "cannot coerce '{}' to {:?}",
                        value.type_name(),
                        target
                    )))?;
                    stack.push(coerced);
                }
                Instr::Pop => {
                    let _ = pop_value(&mut stack, "pop")?;
                }
                _ => {
                    return Err(VmError::InvalidOperation(
                        "statement instruction used in expression context".to_string(),
                    ))
                }
            }
        }
        stack.pop().ok_or_else(|| VmError::InvalidOperation("expression did not produce a value".to_string()))
    }

    fn run_block(&mut self, code: &[Instr]) -> Result<ControlFlow, VmError> {
        self.scopes.push(ScopeFrame::new());
        let result = self.run_code(code);
        self.scopes.pop();
        result
    }

    fn run_use(&mut self, path: &str, namespace: Option<&str>) -> Result<(), VmError> {
        let full = self.base_dir.join(path);
        let module_name = namespace.map(|s| s.to_string()).or_else(|| module_name_from_path(path)).ok_or_else(|| VmError::ImportError(format!("cannot derive module name from import path: {path}")))?;
        let program = self.load_import_program(&full)?;
        let mut child = Vm::new(parent_of(&full));
        child.scopes = self.scopes.clone();
        child.functions = self.functions.clone();
        child.user_modules = self.user_modules.clone();
        child.import_cache = self.import_cache.clone();
        child.run_program(&program)?;
        let state = child.into_module_state();
        self.scopes = state.scopes.clone();
        self.functions = state.functions.clone();
        self.user_modules.insert(module_name, state);
        Ok(())
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, VmError> {
        if let Some(func) = self.stdlib.builtins.get(name) {
            return func(args).map_err(|e| VmError::InvalidOperation(e.to_string()));
        }
        let f = self.functions.get(name).cloned().ok_or_else(|| VmError::UndefinedVariable(name.to_string()))?;
        self.call_compiled_function(name, f, args)
    }

    fn call_compiled_function(&mut self, name: &str, f: CompiledFunction, args: Vec<Value>) -> Result<Value, VmError> {
        if args.len() != f.params.len() {
            return Err(VmError::InvalidOperation(format!("function '{name}' expects {} args, got {}", f.params.len(), args.len())));
        }
        self.scopes.push(ScopeFrame::new());
        for (param, arg) in f.params.iter().zip(args.into_iter()) {
            let val = if let Some(ann) = param.annotation {
                arg.coerce_to(ann).ok_or_else(|| VmError::TypeError(format!("cannot coerce arg '{}' to {:?}", arg.type_name(), ann)))?
            } else { arg };
            self.current_scope_mut().define(param.name.clone(), val);
        }
        let run_result = self.run_code(&f.chunk.code);
        self.scopes.pop();
        let result = match run_result? { ControlFlow::Continue => Value::Null, ControlFlow::Return(v) => v };
        if let Some(ann) = f.ret_type {
            result.coerce_to(ann).ok_or_else(|| VmError::TypeError(format!("cannot coerce return '{}' to {:?}", result.type_name(), ann)))
        } else { Ok(result) }
    }

    fn load_import_program(&mut self, full: &Path) -> Result<Program, VmError> {
        let key = fs::canonicalize(full).unwrap_or_else(|_| full.to_path_buf());
        let modified = fs::metadata(full).and_then(|m| m.modified()).ok();
        if let Some(cached) = self.import_cache.get(&key) {
            if cached.modified == modified { return Ok(cached.program.clone()); }
        }
        let content = fs::read_to_string(full).map_err(|e| VmError::ImportError(format!("{}: {e}", full.display())))?;
        let tokens = lexer::tokenize(&content).map_err(|e| VmError::ParseError(e.to_string()))?;
        let program = parser::parse(tokens).map_err(|e| VmError::ParseError(e.to_string()))?;
        self.import_cache.insert(key, CachedImport { modified, program: program.clone() });
        Ok(program)
    }

    fn call_module(&mut self, module: &str, function: &str, args: Vec<Value>) -> Result<Value, VmError> {
        if let Some(state) = self.user_modules.get(module).cloned() {
            return self.call_user_module_function(module, function, args, state);
        }
        self.stdlib.call_module(module, function, args).map_err(|e| VmError::ModuleError(e.to_string()))
    }

    fn call_std_module(&mut self, module: &str, function: &str, args: Vec<Value>) -> Result<Value, VmError> {
        self.stdlib.call_module(module, function, args).map_err(|e| VmError::ModuleError(e.to_string()))
    }

    fn call_user_module_function(&mut self, module: &str, function: &str, args: Vec<Value>, state: ModuleState) -> Result<Value, VmError> {
        let mut child = Vm::from_module_state(state);
        let result = child.call_function(function, args);
        if result.is_ok() { self.user_modules.insert(module.to_string(), child.into_module_state()); }
        result
    }

    fn current_scope_mut(&mut self) -> &mut ScopeFrame { self.scopes.last_mut().expect("scope stack is non-empty") }
    fn get_var(&self, name: &str) -> Result<Value, VmError> { for scope in self.scopes.iter().rev() { if let Some(v) = scope.get(name) { return Ok(v.clone()); } } Err(VmError::UndefinedVariable(name.to_string())) }
    fn set_var(&mut self, name: &str, value: Value) -> Result<(), VmError> { for idx in (0..self.scopes.len()).rev() { if self.scopes[idx].names.contains_key(name) { self.scopes[idx].set(name, value); return Ok(()); } } Err(VmError::UndefinedVariable(name.to_string())) }
    fn eval_index(&self, obj: Value, idx: Value) -> Result<Value, VmError> { match (obj, idx) { (Value::List(items), Value::Int(i)) => items.get(i as usize).cloned().ok_or_else(|| VmError::InvalidOperation("list index out of bounds".to_string())), (Value::Json(map), Value::Str(key)) => Ok(map.get(key.as_ref()).cloned().unwrap_or(Value::Null)), _ => Err(VmError::TypeError("indexing requires list[int] or json[str]".to_string())), } }
    fn apply_unary(&self, op: UnaryOp, value: &Value) -> Result<Value, VmError> { match op { UnaryOp::Not => Ok(Value::Bool(!value.is_truthy())), UnaryOp::Neg => match value { Value::Int(v) => Ok(Value::Int(-v)), Value::Float(v) => Ok(Value::Float(-v)), _ => Err(VmError::TypeError("unary '-' expects int or float".to_string())), }, } }
    fn apply_binop(&self, left: &Value, op: BinaryOp, right: &Value) -> Result<Value, VmError> { /* same as before */ match op { BinaryOp::Add => match (left, right) { (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)), (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)), (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)), (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)), (Value::Str(a), Value::Str(b)) => { let mut s = String::with_capacity(a.len() + b.len()); s.push_str(a); s.push_str(b); Ok(Value::Str(Arc::from(s))) }, _ => Err(VmError::TypeError("invalid '+' operands".to_string())), }, BinaryOp::Sub => num_binop(left, right, |a, b| a - b, |a, b| a - b), BinaryOp::Mul => num_binop(left, right, |a, b| a * b, |a, b| a * b), BinaryOp::Div => num_binop(left, right, |a, b| a / b, |a, b| a / b), BinaryOp::FloorDiv => match (left, right) { (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)), _ => Err(VmError::TypeError("'//' expects int operands".to_string())), }, BinaryOp::Mod => match (left, right) { (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)), _ => Err(VmError::TypeError("'%' expects int operands".to_string())), }, BinaryOp::Pow => match (left, right) { (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.pow(*b as u32))), (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))), (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).powf(*b))), (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.powf(*b as f64))), _ => Err(VmError::TypeError("'**' expects numeric operands".to_string())), }, BinaryOp::Eq => Ok(Value::Bool(value_eq(left, right))), BinaryOp::Ne => Ok(Value::Bool(!value_eq(left, right))), BinaryOp::Lt => cmp_binop(left, right, |a, b| a < b), BinaryOp::Gt => cmp_binop(left, right, |a, b| a > b), BinaryOp::Le => cmp_binop(left, right, |a, b| a <= b), BinaryOp::Ge => cmp_binop(left, right, |a, b| a >= b), BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())), BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())), } }
    fn into_module_state(self) -> ModuleState { ModuleState { scopes: self.scopes, functions: self.functions, user_modules: self.user_modules, base_dir: self.base_dir } }
    fn from_module_state(state: ModuleState) -> Self { Self { scopes: state.scopes, functions: state.functions, user_modules: state.user_modules, import_cache: HashMap::new(), stdlib: stdlib::Stdlib::new(), base_dir: state.base_dir } }
}

enum ControlFlow { Continue, Return(Value) }

fn pop_value(stack: &mut Vec<Value>, context: &str) -> Result<Value, VmError> { stack.pop().ok_or_else(|| VmError::InvalidOperation(format!("stack underflow during {context}"))) }
fn pop_args(stack: &mut Vec<Value>, argc: usize, context: &str) -> Result<Vec<Value>, VmError> { if stack.len() < argc { return Err(VmError::InvalidOperation(format!("stack underflow during {context}"))); } let mut args = Vec::with_capacity(argc); for _ in 0..argc { args.push(stack.pop().expect("length checked above")); } args.reverse(); Ok(args) }
fn value_eq(a: &Value, b: &Value) -> bool { match (a, b) { (Value::Int(x), Value::Int(y)) => x == y, (Value::Float(x), Value::Float(y)) => x == y, (Value::Int(x), Value::Float(y)) => *x as f64 == *y, (Value::Float(x), Value::Int(y)) => *x == *y as f64, (Value::Str(x), Value::Str(y)) => x == y, (Value::Bool(x), Value::Bool(y)) => x == y, (Value::Null, Value::Null) => true, _ => false } }
fn num_binop(left: &Value, right: &Value, int_op: fn(i64, i64) -> i64, float_op: fn(f64, f64) -> f64) -> Result<Value, VmError> { match (left, right) { (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))), (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a, *b))), (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(*a as f64, *b))), (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(*a, *b as f64))), _ => Err(VmError::TypeError("numeric operands required".to_string())), } }
fn cmp_binop(left: &Value, right: &Value, op: fn(f64, f64) -> bool) -> Result<Value, VmError> { match (left, right) { (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(op(*a as f64, *b as f64))), (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(op(*a, *b))), (Value::Int(a), Value::Float(b)) => Ok(Value::Bool(op(*a as f64, *b))), (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(op(*a, *b as f64))), _ => Err(VmError::TypeError("comparison requires numeric operands".to_string())), } }
fn parent_of(path: &Path) -> PathBuf { path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf() }
fn module_name_from_path(path: &str) -> Option<String> { let stem = Path::new(path).file_stem()?.to_str()?; Some(stem.to_string()) }
