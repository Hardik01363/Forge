use crate::ir::{Instruction, IrFunction, IrProgram, Val};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86Reg {
    AX, //accumulator/return value register (%eax / %rax)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X86Operand {
    Imm(i32),
    Reg(X86Reg),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X86Instruction {
    Mov {
        src: X86Operand,
        dst: X86Operand,
    },
    Ret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X86Function {
    pub name: String,
    pub instructions: Vec<X86Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X86Program {
    pub functions: Vec<X86Function>,
}
