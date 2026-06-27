use ast::{AssignOp, BinaryOp, Param, TypeAnnotation, UnaryOp, Value};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub code: Vec<OpCode>,
}

impl Chunk {
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FunctionBytecode {
    pub params: Vec<Param>,
    pub ret_type: Option<TypeAnnotation>,
    pub chunk: Chunk,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum TextPart {
    Text(String),
    Var(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum OpCode {
    Const(Value),
    LoadLocal(String),
    Push(Value),
    LoadVar(String),
    Unary(UnaryOp),
    Binary(BinaryOp),
    BuildList(usize),
    BuildMap(Vec<String>),
    MakeList(usize),
    MakeJson(Vec<String>),
    Interpolate(Vec<TextPart>),
    CallLocal { name: String, argc: usize },
    CallModule { module: String, function: String, argc: usize },
    CallStd { module: String, function: String, argc: usize },
    CallFunction { name: String, argc: usize },
    CallStdModule { module: String, function: String, argc: usize },
    IndexGet,
    GetIndex,
    Cast(TypeAnnotation),
    Coerce(TypeAnnotation),
    Pop,
    StoreLocal(String),
    DefineVar(String),
    UpdateLocal { name: String, op: AssignOp },
    AssignVar { name: String, op: AssignOp },
    DefineFunction { name: String, function: FunctionBytecode },
    If {
        cond: Chunk,
        then_body: Chunk,
        elifs: Vec<(Chunk, Chunk)>,
        else_body: Option<Chunk>,
    },
    While {
        cond: Chunk,
        body: Chunk,
    },
    ForRange {
        start: Chunk,
        end: Chunk,
        var: String,
        body: Chunk,
    },
    ForEach {
        iter: Chunk,
        var: String,
        body: Chunk,
    },
    Jump(usize),
    JumpIfFalse(usize),
    Loop(usize),
    Return(Option<Chunk>),
    Use { path: String, namespace: Option<String> },
}

pub type Instr = OpCode;
pub type CompiledFunction = FunctionBytecode;
pub type InterpPart = TextPart;
pub type Bytecode = Chunk;
