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

pub struct Codegen;

impl Codegen {
    //translates the target-independent IR to x86-64 instruction AST.
    pub fn generate_x86(ir: &IrProgram) -> X86Program {
        let mut functions = Vec::new();

        for ir_fn in &ir.functions {
            let mut x86_instructions = Vec::new();

            for inst in &ir_fn.instructions {
                match inst {
                    Instruction::Return(val) => {
                        let src = match val {
                            Val::Constant(c) => X86Operand::Imm(*c as i32),
                            Val::Var(v) => {
                                todo!("will implement variable return later: {v}");
                            }
                        };
                        //as Per System V ABI: function return value is placed in EAX / RAX
                        x86_instructions.push(X86Instruction::Mov {
                            src,
                            dst: X86Operand::Reg(X86Reg::AX),
                        });
                        x86_instructions.push(X86Instruction::Ret);
                    }
                    Instruction::Unary { .. }
                    | Instruction::Binary { .. }
                    | Instruction::Copy { .. } => {
                        todo!("will implement operator instruction selection in later");
                    }
                }
            }

            functions.push(X86Function {
                name: ir_fn.name.clone(),
                instructions: x86_instructions,
            });
        }

        X86Program { functions }
    }

    //emits standard GNU Assembler (GAS / AT&T) syntax text from the x86 program.
    pub fn emit_assembly(program: &X86Program) -> String {
        let mut out = String::new();

        for func in &program.functions {
            //.globl directive makes the symbol visible to the linker (ld): imp note to self, i keep foregtting this :P
            writeln!(out, "    .globl {}", func.name).unwrap();
            writeln!(out, "{}:", func.name).unwrap();

            for inst in &func.instructions {
                match inst {
                    X86Instruction::Mov { src, dst } => {
                        writeln!(out, "    movl {}, {}", Self::format_operand(src), Self::format_operand(dst)).unwrap();
                    }
                    X86Instruction::Ret => {
                        writeln!(out, "    ret").unwrap();
                    }
                }
            }
        }

        //standard ELF non-executable stack note to prevent the pesky linker warnings
        writeln!(out, "    .section .note.GNU-stack,\"\",@progbits").unwrap();

        out
    }

    fn format_operand(op: &X86Operand) -> String {
        match op {
            X86Operand::Imm(val) => format!("${val}"),
            X86Operand::Reg(X86Reg::AX) => "%eax".to_string(),
        }
    }
}
