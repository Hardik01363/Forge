use crate::ir::{Instruction, IrProgram, Val};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86Reg {
    AX,  //accumulator/return value register (%eax / %rax)
    R10, //scratch register to fix up mv and other limitations (%r10d / %r10)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X86Operand {
    Imm(i32),
    Reg(X86Reg),
    Stack(i32), //offset from %rbp
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86UnaryOp {
    Neg, //negl
    Not, //notl
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X86Instruction {
    Mov { src: X86Operand, dst: X86Operand },
    Unary { op: X86UnaryOp, dst: X86Operand },
    //i will implement logical NOT as : cmpl $0, dst; sete %al; movzbl %al, dst
    //i wont use any jump instructions for implementing logical NOT
    Cmp { src: X86Operand, dst: X86Operand },
    SetE(X86Reg), //sete %al (AX)
    MovZeroExtend { src_reg: X86Reg, dst: X86Operand },
    //stack frame allocation / deallocation
    AllocateStack(i32), // subq $N, %rsp (function prologue)
    DeallocateStack,    // movq %rbp, %rsp; popq %rbp (function epilogue)
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
                        writeln!(
                            out,
                            "    movl {}, {}",
                            Self::format_operand(src),
                            Self::format_operand(dst)
                        )
                        .unwrap();
                    }
                    X86Instruction::Ret => {
                        writeln!(out, "    ret").unwrap();
                    }
                }
            }
        }

        //standard ELF non-executable stack note to prevent the pesky linker warnings
        //tells the Linux linker and kernel that our code does not require an executable stack, silencing modern GCC/LD security warnings.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::IrLowerer;
    use crate::parser::Parser;

    #[test]
    fn generates_x86_ast_for_return_constant() {
        let src = "int main(void) { return 42; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);
        let x86 = Codegen::generate_x86(&ir);

        assert_eq!(x86.functions.len(), 1);
        let f = &x86.functions[0];
        assert_eq!(f.name, "main");
        assert_eq!(
            f.instructions,
            vec![
                X86Instruction::Mov {
                    src: X86Operand::Imm(42),
                    dst: X86Operand::Reg(X86Reg::AX),
                },
                X86Instruction::Ret,
            ]
        );
    }

    #[test]
    fn emits_valid_gas_assembly_string() {
        let src = "int main(void) { return 42; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);
        let x86 = Codegen::generate_x86(&ir);
        let asm = Codegen::emit_assembly(&x86);

        let expected = concat!(
            "    .globl main\n",
            "main:\n",
            "    movl $42, %eax\n",
            "    ret\n",
            "    .section .note.GNU-stack,\"\",@progbits\n"
        );
        assert_eq!(asm, expected);
    }
}
