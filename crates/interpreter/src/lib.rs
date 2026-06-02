use ast::{AssignOp, BinaryOp, Expr, Param, Program, Stmt, TypeAnnotation, UnaryOp, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
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

#[derive(Clone)]
struct FunctionDef {
    params: Vec<Param>,
    ret_type: Option<TypeAnnotation>,
    body: Vec<Stmt>,
}

#[derive(Clone)]
struct ModuleState {
    scopes: Vec<HashMap<String, Value>>,
    functions: HashMap<String, FunctionDef>,
    user_modules: HashMap<String, ModuleState>,
    base_dir: PathBuf,
}

pub struct Interpreter {
    scopes: Vec<HashMap<String, Value>>,
    functions: HashMap<String, FunctionDef>,
    user_modules: HashMap<String, ModuleState>,
    stdlib: stdlib::Stdlib,
    base_dir: PathBuf,
}

impl Interpreter {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            user_modules: HashMap::new(),
            stdlib: stdlib::Stdlib::new(),
            base_dir: base_dir.into(),
        }
    }

    pub fn execute_program(&mut self, program: &Program) -> Result<(), RuntimeError> {
        for stmt in &program.statements {
            let flow = self.eval_stmt(stmt)?;
            if matches!(flow, ControlFlow::Return(_)) {
                return Err(RuntimeError::InvalidOperation(
                    "return outside of function".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> Result<ControlFlow, RuntimeError> {
        match stmt {
            Stmt::Let {
                name,
                annotation,
                value,
            } => self.eval_let_stmt(name, annotation, value),
            Stmt::Assign { target, op, value } => self.eval_assign_stmt(target, *op, value),
            Stmt::If {
                cond,
                then_block,
                elif_blocks,
                else_block,
            } => self.eval_if_stmt(cond, then_block, elif_blocks, else_block),
            Stmt::While { cond, body } => self.eval_while_stmt(cond, body),
            Stmt::ForRange {
                var,
                start,
                end,
                body,
            } => self.eval_for_range_stmt(var, start, end, body),
            Stmt::ForEach { var, iter, body } => self.eval_for_each_stmt(var, iter, body),
            Stmt::FnDef {
                name,
                params,
                ret_type,
                body,
            } => self.eval_fn_def_stmt(name, params, *ret_type, body),
            Stmt::Return(expr) => self.eval_return_stmt(expr),
            Stmt::Use(path) => self.eval_use_stmt(path),
            Stmt::Expr(expr) => {
                self.eval_expr(expr)?;
                Ok(ControlFlow::Continue)
            } // EXTENSION POINT: add new statement evaluators in this dispatch.
        }
    }

    fn eval_let_stmt(
        &mut self,
        name: &str,
        annotation: &Option<TypeAnnotation>,
        value: &Expr,
    ) -> Result<ControlFlow, RuntimeError> {
        let mut val = self.eval_expr(value)?;
        if let Some(ann) = annotation {
            val = val.coerce_to(*ann).ok_or_else(|| {
                RuntimeError::TypeError(format!(
                    "cannot coerce '{}' to annotation {:?}",
                    val.type_name(),
                    ann
                ))
            })?;
        }
        self.current_scope_mut().insert(name.to_string(), val);
        Ok(ControlFlow::Continue)
    }

    fn eval_assign_stmt(
        &mut self,
        target: &Expr,
        op: AssignOp,
        value: &Expr,
    ) -> Result<ControlFlow, RuntimeError> {
        match target {
            Expr::Var(name) => {
                let rhs = self.eval_expr(value)?;
                let current = self.get_var(name)?;
                let next = match op {
                    AssignOp::Assign => rhs,
                    AssignOp::AddAssign => self.apply_binop(&current, BinaryOp::Add, &rhs)?,
                    AssignOp::SubAssign => self.apply_binop(&current, BinaryOp::Sub, &rhs)?,
                    AssignOp::MulAssign => self.apply_binop(&current, BinaryOp::Mul, &rhs)?,
                    AssignOp::DivAssign => self.apply_binop(&current, BinaryOp::Div, &rhs)?,
                };
                self.set_var(name, next)?;
                Ok(ControlFlow::Continue)
            }
            _ => Err(RuntimeError::InvalidOperation(
                "assignment target must be a variable".to_string(),
            )),
        }
    }

    fn eval_if_stmt(
        &mut self,
        cond: &Expr,
        then_block: &[Stmt],
        elif_blocks: &[(Expr, Vec<Stmt>)],
        else_block: &Option<Vec<Stmt>>,
    ) -> Result<ControlFlow, RuntimeError> {
        if self.eval_expr(cond)?.is_truthy() {
            return self.eval_block(then_block);
        }
        for (elif_cond, block) in elif_blocks {
            if self.eval_expr(elif_cond)?.is_truthy() {
                return self.eval_block(block);
            }
        }
        if let Some(block) = else_block {
            return self.eval_block(block);
        }
        Ok(ControlFlow::Continue)
    }

    fn eval_while_stmt(&mut self, cond: &Expr, body: &[Stmt]) -> Result<ControlFlow, RuntimeError> {
        while self.eval_expr(cond)?.is_truthy() {
            if let ControlFlow::Return(v) = self.eval_block(body)? {
                return Ok(ControlFlow::Return(v));
            }
        }
        Ok(ControlFlow::Continue)
    }

    fn eval_for_range_stmt(
        &mut self,
        var: &str,
        start: &Expr,
        end: &Expr,
        body: &[Stmt],
    ) -> Result<ControlFlow, RuntimeError> {
        let start_v = self.eval_expr(start)?;
        let end_v = self.eval_expr(end)?;
        let (s, e) = match (start_v, end_v) {
            (Value::Int(s), Value::Int(e)) => (s, e),
            _ => {
                return Err(RuntimeError::TypeError(
                    "for range requires int bounds".to_string(),
                ))
            }
        };

        for i in s..e {
            self.current_scope_mut()
                .insert(var.to_string(), Value::Int(i));
            if let ControlFlow::Return(v) = self.eval_block(body)? {
                return Ok(ControlFlow::Return(v));
            }
        }
        Ok(ControlFlow::Continue)
    }

    fn eval_for_each_stmt(
        &mut self,
        var: &str,
        iter: &Expr,
        body: &[Stmt],
    ) -> Result<ControlFlow, RuntimeError> {
        let iterable = self.eval_expr(iter)?;
        let items = match iterable {
            Value::List(items) => items,
            _ => {
                return Err(RuntimeError::TypeError(
                    "for each requires list iterable".to_string(),
                ))
            }
        };
        for item in items {
            self.current_scope_mut().insert(var.to_string(), item);
            if let ControlFlow::Return(v) = self.eval_block(body)? {
                return Ok(ControlFlow::Return(v));
            }
        }
        Ok(ControlFlow::Continue)
    }

    fn eval_fn_def_stmt(
        &mut self,
        name: &str,
        params: &[Param],
        ret_type: Option<TypeAnnotation>,
        body: &[Stmt],
    ) -> Result<ControlFlow, RuntimeError> {
        self.functions.insert(
            name.to_string(),
            FunctionDef {
                params: params.to_vec(),
                ret_type,
                body: body.to_vec(),
            },
        );
        Ok(ControlFlow::Continue)
    }

    fn eval_return_stmt(&mut self, expr: &Option<Expr>) -> Result<ControlFlow, RuntimeError> {
        let value = if let Some(expr) = expr {
            self.eval_expr(expr)?
        } else {
            Value::Null
        };
        Ok(ControlFlow::Return(value))
    }

    fn eval_use_stmt(&mut self, path: &str) -> Result<ControlFlow, RuntimeError> {
        let full = self.base_dir.join(path);
        let content = fs::read_to_string(&full)
            .map_err(|e| RuntimeError::ImportError(format!("{}: {e}", full.display())))?;
        let tokens =
            lexer::tokenize(&content).map_err(|e| RuntimeError::ParseError(e.to_string()))?;
        let program = parser::parse(tokens).map_err(|e| RuntimeError::ParseError(e.to_string()))?;
        let module_name = module_name_from_path(path).ok_or_else(|| {
            RuntimeError::ImportError(format!(
                "cannot derive module name from import path: {path}"
            ))
        })?;

        let mut child = Interpreter::new(parent_of(&full));
        child.scopes = self.scopes.clone();
        child.functions = self.functions.clone();
        child.user_modules = self.user_modules.clone();
        child.execute_program(&program)?;

        let module_state = child.into_module_state();
        self.scopes = module_state.scopes.clone();
        self.functions = module_state.functions.clone();
        self.user_modules.insert(module_name, module_state);
        Ok(ControlFlow::Continue)
    }

    fn eval_block(&mut self, stmts: &[Stmt]) -> Result<ControlFlow, RuntimeError> {
        self.scopes.push(HashMap::new());
        for stmt in stmts {
            let flow = self.eval_stmt(stmt)?;
            if let ControlFlow::Return(_) = flow {
                self.scopes.pop();
                return Ok(flow);
            }
        }
        self.scopes.pop();
        Ok(ControlFlow::Continue)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Value(v) => Ok(v.clone()),
            Expr::Var(name) => self.get_var(name),
            Expr::InterpolatedString(raw) => self.eval_interpolated(raw),
            Expr::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.eval_expr(item)?);
                }
                Ok(Value::List(out))
            }
            Expr::Json(entries) => {
                let mut out = BTreeMap::new();
                for (k, vexpr) in entries {
                    out.insert(k.clone(), self.eval_expr(vexpr)?);
                }
                Ok(Value::Json(out))
            }
            Expr::Unary { op, expr } => {
                let val = self.eval_expr(expr)?;
                self.apply_unary(*op, &val)
            }
            Expr::Binary { left, op, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.apply_binop(&l, *op, &r)
            }
            Expr::Call { callee, args } => {
                let name = match &**callee {
                    Expr::Var(name) => name.clone(),
                    _ => {
                        return Err(RuntimeError::InvalidOperation(
                            "call target must be an identifier".to_string(),
                        ))
                    }
                };
                let mut evaled = Vec::with_capacity(args.len());
                for arg in args {
                    evaled.push(self.eval_expr(arg)?);
                }
                self.call_function(&name, evaled)
            }
            Expr::ModuleCall {
                module,
                function,
                args,
            } => {
                let mut evaled = Vec::with_capacity(args.len());
                for arg in args {
                    evaled.push(self.eval_expr(arg)?);
                }
                self.call_module(module, function, evaled)
            }
            Expr::Index { object, index } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                self.eval_index(obj, idx)
            }
        }
    }

    fn eval_index(&self, obj: Value, idx: Value) -> Result<Value, RuntimeError> {
        match (obj, idx) {
            (Value::List(items), Value::Int(i)) => {
                items.get(i as usize).cloned().ok_or_else(|| {
                    RuntimeError::InvalidOperation("list index out of bounds".to_string())
                })
            }
            (Value::Json(map), Value::Str(key)) => {
                Ok(map.get(&key).cloned().unwrap_or(Value::Null))
            }
            _ => Err(RuntimeError::TypeError(
                "indexing requires list[int] or json[str]".to_string(),
            )),
        }
    }

    fn eval_interpolated(&self, raw: &str) -> Result<Value, RuntimeError> {
        let mut out = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut name = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '}' {
                        break;
                    }
                    name.push(next);
                }
                let val = self.get_var(name.trim())?;
                out.push_str(&val.to_pretty_string());
            } else {
                out.push(ch);
            }
        }
        Ok(Value::Str(out))
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
            // EXTENSION POINT: add unary operators here.
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
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
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
            // EXTENSION POINT: add binary operators here.
        }
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
        if let Ok(v) = self.stdlib.call_builtin(name, args.clone()) {
            return Ok(v);
        }

        let f = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::UndefinedVariable(name.to_string()))?;

        if args.len() != f.params.len() {
            return Err(RuntimeError::InvalidOperation(format!(
                "function '{name}' expects {} args, got {}",
                f.params.len(),
                args.len()
            )));
        }

        self.scopes.push(HashMap::new());
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
            self.current_scope_mut().insert(param.name.clone(), val);
        }

        let mut ret = Value::Null;
        for stmt in &f.body {
            match self.eval_stmt(stmt)? {
                ControlFlow::Continue => {}
                ControlFlow::Return(v) => {
                    ret = v;
                    break;
                }
            }
        }
        self.scopes.pop();

        if let Some(ann) = f.ret_type {
            ret.coerce_to(ann).ok_or_else(|| {
                RuntimeError::TypeError(format!(
                    "cannot coerce return '{}' to {:?}",
                    ret.type_name(),
                    ann
                ))
            })
        } else {
            Ok(ret)
        }
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

    fn current_scope_mut(&mut self) -> &mut HashMap<String, Value> {
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
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        Err(RuntimeError::UndefinedVariable(name.to_string()))
    }

    fn from_module_state(state: ModuleState) -> Self {
        Self {
            scopes: state.scopes,
            functions: state.functions,
            user_modules: state.user_modules,
            stdlib: stdlib::Stdlib::new(),
            base_dir: state.base_dir,
        }
    }

    fn into_module_state(self) -> ModuleState {
        ModuleState {
            scopes: self.scopes,
            functions: self.functions,
            user_modules: self.user_modules,
            base_dir: self.base_dir,
        }
    }
}

enum ControlFlow {
    Continue,
    Return(Value),
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
