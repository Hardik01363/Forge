use crate::ast::{AstArena, BinaryOp, ExprId, ExprKind, Program, Statement, UnaryOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    Constant(i64),
    Var(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrUnaryOp {
    Negate,
    Complement,
    Not,
}

impl From<UnaryOp> for IrUnaryOp {
    fn from(op: UnaryOp) -> Self {
        match op {
            UnaryOp::Negate => IrUnaryOp::Negate,
            UnaryOp::Complement => IrUnaryOp::Complement,
            UnaryOp::Not => IrUnaryOp::Not,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

impl From<BinaryOp> for IrBinaryOp {
    fn from(op: BinaryOp) -> Self {
        match op {
            BinaryOp::Add => IrBinaryOp::Add,
            BinaryOp::Subtract => IrBinaryOp::Subtract,
            BinaryOp::Multiply => IrBinaryOp::Multiply,
            BinaryOp::Divide => IrBinaryOp::Divide,
            BinaryOp::Modulo => IrBinaryOp::Modulo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Return(Val),
    Unary {
        op: IrUnaryOp,
        src: Val,
        dst: Val,
    },
    Binary {
        op: IrBinaryOp,
        left: Val,
        right: Val,
        dst: Val,
    },
    Copy {
        src: Val,
        dst: Val,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrFunction {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
}
