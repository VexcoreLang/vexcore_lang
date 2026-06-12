use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeAnnotation {
    Int,
    Float,
    Str,
    Bool,
    Null,
    List,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    Bool(bool),
    Null,
    List(Arc<[Value]>),
    Json(Arc<BTreeMap<String, Value>>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::Bool(_) => "bool",
            Value::Null => "null",
            Value::List(_) => "list",
            Value::Json(_) => "json",
            // EXTENSION POINT: add type name for new Value variants here.
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(v) => *v,
            Value::Null => false,
            Value::Int(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::Str(v) => !v.is_empty(),
            Value::List(v) => !v.is_empty(),
            Value::Json(v) => !v.is_empty(),
            // EXTENSION POINT: add truthiness for new Value variants here.
        }
    }

    pub fn to_pretty_string(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::Float(v) => {
                if v.fract() == 0.0 {
                    format!("{v:.1}")
                } else {
                    v.to_string()
                }
            }
            Value::Str(v) => v.to_string(),
            Value::Bool(v) => v.to_string(),
            Value::Null => "null".to_string(),
            Value::List(values) => {
                let inner = values
                    .iter()
                    .map(|x| x.to_pretty_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            Value::Json(map) => {
                serde_json::to_string(map.as_ref()).unwrap_or_else(|_| "{}".to_string())
            } // EXTENSION POINT: add stringification for new Value variants here.
        }
    }

    pub fn coerce_to(&self, target: TypeAnnotation) -> Option<Value> {
        match target {
            TypeAnnotation::Int => match self {
                Value::Int(v) => Some(Value::Int(*v)),
                Value::Float(v) => Some(Value::Int(*v as i64)),
                Value::Str(s) => s.parse::<i64>().ok().map(Value::Int),
                Value::Bool(b) => Some(Value::Int(if *b { 1 } else { 0 })),
                _ => None,
            },
            TypeAnnotation::Float => match self {
                Value::Int(v) => Some(Value::Float(*v as f64)),
                Value::Float(v) => Some(Value::Float(*v)),
                Value::Str(s) => s.parse::<f64>().ok().map(Value::Float),
                Value::Bool(b) => Some(Value::Float(if *b { 1.0 } else { 0.0 })),
                _ => None,
            },
            TypeAnnotation::Str => Some(Value::Str(self.to_pretty_string().into())),
            TypeAnnotation::Bool => Some(Value::Bool(self.is_truthy())),
            TypeAnnotation::Null => Some(Value::Null),
            TypeAnnotation::List => match self {
                Value::List(v) => Some(Value::List(v.clone())),
                _ => None,
            },
            TypeAnnotation::Json => match self {
                Value::Json(v) => Some(Value::Json(v.clone())),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        annotation: Option<TypeAnnotation>,
        value: Expr,
    },
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
    },
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        elif_blocks: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    ForRange {
        var: String,
        start: Expr,
        end: Expr,
        body: Vec<Stmt>,
    },
    ForEach {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    FnDef {
        name: String,
        params: Vec<Param>,
        ret_type: Option<TypeAnnotation>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Use {
        path: String,
        namespace: Option<String>,
    },
    Expr(Expr),
    // EXTENSION POINT: add new statement variants here.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub annotation: Option<TypeAnnotation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Value(Value),
    Var(String),
    InterpolatedString(String),
    List(Vec<Expr>),
    Json(Vec<(String, Expr)>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    ModuleCall {
        module: String,
        function: String,
        args: Vec<Expr>,
    },
    StdModuleCall {
        module: String,
        function: String,
        args: Vec<Expr>,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}
