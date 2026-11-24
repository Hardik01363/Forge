use crate::ir::{Instruction, IrProgram, Val};
use std::collections::HashMap;
use std::fmt::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86Reg {
    AX,  //accumulator/return value register (%eax / %rax)
    DX,  //dividend high bits, remainder after idiv (%edx / %rdx)
    R10, //scratch register to fix up mv and other limitations (%r10d / %r10)
    R11, //2nd scratch register for instruction fixup (%r11d / %r11)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X86Operand {
    Imm(i32),
    Reg(X86Reg),
    Stack(i32), //offset from %rbp
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86UnaryOp {
    Neg, //neg
    Not, //not
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86BinaryOp {
    Add, //add
    Sub, //sub
    Mul, //imul
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86CondCode {
    E,  //equal to (== or jump if zero)
    Ne, //not equal to (!= or jump if not zero)
    L,  //less than (<)
    Le, //less than or equal to (<=)
    G,  //greater than (>)
    Ge, //greater than or equal to (>=)
}

impl fmt::Display for X86CondCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            X86CondCode::E => write!(f, "e"),
            X86CondCode::Ne => write!(f, "ne"),
            X86CondCode::L => write!(f, "l"),
            X86CondCode::Le => write!(f, "le"),
            X86CondCode::G => write!(f, "g"),
            X86CondCode::Ge => write!(f, "ge"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X86Instruction {
    Mov {
        src: X86Operand,
        dst: X86Operand,
    },
    Unary {
        op: X86UnaryOp,
        dst: X86Operand,
    },
    Binary {
        op: X86BinaryOp,
        src: X86Operand,
        dst: X86Operand,
    },
    Cdq,              //sign-extends %eax into %edx:%eax
    Idiv(X86Operand), //signed divide %edx:%eax by operand
    //i will implement logical NOT as : cmpl $0, dst; sete %al; movzbl %al, dst
    //i wont use any jump instructions for implementing logical NOT
    Cmp {
        src: X86Operand,
        dst: X86Operand,
    },
    SetCc(X86CondCode, X86Reg), // set<cc> %al (AX)
    MovZeroExtend {
        src_reg: X86Reg,
        dst: X86Operand,
    },
    Jmp(String),
    JmpCc(X86CondCode, String),
    Label(String),
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
    //translates target-independent IR to x86-64 instruction AST with stack-allocated variables.
    pub fn generate_x86(ir: &IrProgram) -> X86Program {
        let mut functions = Vec::new();
        for ir_fn in &ir.functions {
            let mut var_map: HashMap<String, i32> = HashMap::new();
            let mut stack_offset = 0;
            //helper to get/allocate a stack offset for a variable
            let get_operand =
                |val: &Val, stack_off: &mut i32, map: &mut HashMap<String, i32>| -> X86Operand {
                    match val {
                        Val::Constant(c) => X86Operand::Imm(*c as i32),
                        Val::Var(name) => {
                            if let Some(&off) = map.get(name) {
                                X86Operand::Stack(off)
                            } else {
                                *stack_off -= 4; //each 32-bit int variable takes 4 bytes
                                map.insert(name.clone(), *stack_off);
                                X86Operand::Stack(*stack_off)
                            }
                        }
                    }
                };
            let mut raw_instructions = Vec::new();
            for inst in &ir_fn.instructions {
                match inst {
                    Instruction::Return(val) => {
                        let src = get_operand(val, &mut stack_offset, &mut var_map);
                        raw_instructions.push(X86Instruction::Mov {
                            src,
                            dst: X86Operand::Reg(X86Reg::AX),
                        });
                        raw_instructions.push(X86Instruction::DeallocateStack);
                        raw_instructions.push(X86Instruction::Ret);
                    }
                    Instruction::Unary { op, src, dst } => {
                        let src_op = get_operand(src, &mut stack_offset, &mut var_map);
                        let dst_op = get_operand(dst, &mut stack_offset, &mut var_map);
                        //in x86, unary ops are in-place: dst = op(src)
                        //first copy src into dst: movl src, dst
                        raw_instructions.push(X86Instruction::Mov {
                            src: src_op,
                            dst: dst_op.clone(),
                        });
                        match op {
                            crate::ir::IrUnaryOp::Negate => {
                                raw_instructions.push(X86Instruction::Unary {
                                    op: X86UnaryOp::Neg,
                                    dst: dst_op,
                                });
                            }
                            crate::ir::IrUnaryOp::Complement => {
                                raw_instructions.push(X86Instruction::Unary {
                                    op: X86UnaryOp::Not,
                                    dst: dst_op,
                                });
                            }
                            crate::ir::IrUnaryOp::Not => {
                                //cmpl $0, dst; sete %al; movzbl %al, dst
                                raw_instructions.push(X86Instruction::Cmp {
                                    src: X86Operand::Imm(0),
                                    dst: dst_op.clone(),
                                });
                                raw_instructions
                                    .push(X86Instruction::SetCc(X86CondCode::E, X86Reg::AX));
                                raw_instructions.push(X86Instruction::MovZeroExtend {
                                    src_reg: X86Reg::AX,
                                    dst: dst_op,
                                });
                            }
                        }
                    }
                    Instruction::Binary {
                        op,
                        left,
                        right,
                        dst,
                    } => {
                        let left_op = get_operand(left, &mut stack_offset, &mut var_map);
                        let right_op = get_operand(right, &mut stack_offset, &mut var_map);
                        let dst_op = get_operand(dst, &mut stack_offset, &mut var_map);

                        match op {
                            crate::ir::IrBinaryOp::Add
                            | crate::ir::IrBinaryOp::Subtract
                            | crate::ir::IrBinaryOp::Multiply => {
                                let bin_op = match op {
                                    crate::ir::IrBinaryOp::Add => X86BinaryOp::Add,
                                    crate::ir::IrBinaryOp::Subtract => X86BinaryOp::Sub,
                                    crate::ir::IrBinaryOp::Multiply => X86BinaryOp::Mul,
                                    _ => unreachable!(),
                                };
                                //copy left into dst: movl left, dst
                                raw_instructions.push(X86Instruction::Mov {
                                    src: left_op,
                                    dst: dst_op.clone(),
                                });
                                //apply binary op: op right, dst
                                raw_instructions.push(X86Instruction::Binary {
                                    op: bin_op,
                                    src: right_op,
                                    dst: dst_op,
                                });
                            }
                            crate::ir::IrBinaryOp::Divide | crate::ir::IrBinaryOp::Modulo => {
                                //move left into EAX: movl left, %eax
                                raw_instructions.push(X86Instruction::Mov {
                                    src: left_op,
                                    dst: X86Operand::Reg(X86Reg::AX),
                                });
                                //sign-extend %eax into %edx:%eax: cdq
                                raw_instructions.push(X86Instruction::Cdq);
                                //divide by right: idivl right
                                raw_instructions.push(X86Instruction::Idiv(right_op));
                                //store result from %eax (quotient) or %edx (remainder) into dst
                                let result_reg = if *op == crate::ir::IrBinaryOp::Divide {
                                    X86Reg::AX
                                } else {
                                    X86Reg::DX
                                };
                                raw_instructions.push(X86Instruction::Mov {
                                    src: X86Operand::Reg(result_reg),
                                    dst: dst_op,
                                });
                            }
                            crate::ir::IrBinaryOp::Equal
                            | crate::ir::IrBinaryOp::NotEqual
                            | crate::ir::IrBinaryOp::LessThan
                            | crate::ir::IrBinaryOp::LessOrEqual
                            | crate::ir::IrBinaryOp::GreaterThan
                            | crate::ir::IrBinaryOp::GreaterOrEqual => {
                                let cond = match op {
                                    crate::ir::IrBinaryOp::Equal => X86CondCode::E,
                                    crate::ir::IrBinaryOp::NotEqual => X86CondCode::Ne,
                                    crate::ir::IrBinaryOp::LessThan => X86CondCode::L,
                                    crate::ir::IrBinaryOp::LessOrEqual => X86CondCode::Le,
                                    crate::ir::IrBinaryOp::GreaterThan => X86CondCode::G,
                                    crate::ir::IrBinaryOp::GreaterOrEqual => X86CondCode::Ge,
                                    _ => unreachable!(),
                                };
                                //cmpl right, left (checks left <cond> right)
                                raw_instructions.push(X86Instruction::Cmp {
                                    src: right_op,
                                    dst: left_op,
                                });
                                //set<cond> %al
                                raw_instructions.push(X86Instruction::SetCc(cond, X86Reg::AX));
                                //movzbl %al, dst
                                raw_instructions.push(X86Instruction::MovZeroExtend {
                                    src_reg: X86Reg::AX,
                                    dst: dst_op,
                                });
                            }
                        }
                    }
                    Instruction::Copy { src, dst } => {
                        let src_op = get_operand(src, &mut stack_offset, &mut var_map);
                        let dst_op = get_operand(dst, &mut stack_offset, &mut var_map);
                        raw_instructions.push(X86Instruction::Mov {
                            src: src_op,
                            dst: dst_op,
                        });
                    }
                    Instruction::Label(label) => {
                        raw_instructions.push(X86Instruction::Label(label.clone()));
                    }
                    Instruction::Jump(target) => {
                        raw_instructions.push(X86Instruction::Jmp(target.clone()));
                    }
                    Instruction::JumpIfZero { condition, target } => {
                        let cond_op = get_operand(condition, &mut stack_offset, &mut var_map);
                        //cmpl $0, cond_op
                        raw_instructions.push(X86Instruction::Cmp {
                            src: X86Operand::Imm(0),
                            dst: cond_op,
                        });
                        //je target
                        raw_instructions
                            .push(X86Instruction::JmpCc(X86CondCode::E, target.clone()));
                    }
                    Instruction::JumpIfNotZero { condition, target } => {
                        let cond_op = get_operand(condition, &mut stack_offset, &mut var_map);
                        //cmpl $0, cond_op
                        raw_instructions.push(X86Instruction::Cmp {
                            src: X86Operand::Imm(0),
                            dst: cond_op,
                        });
                        //jne target
                        raw_instructions
                            .push(X86Instruction::JmpCc(X86CondCode::Ne, target.clone()));
                    }
                }
            }
            //align stack allocation to 16 bytes per System V ABI requirement
            //total_bytes is absolute value of stack_offset
            let total_bytes = -stack_offset;
            let aligned_bytes = (total_bytes + 15) & !15;
            //prepend stack allocation in prologue if stack space was used
            let mut final_instructions = Vec::new();
            if aligned_bytes > 0 {
                final_instructions.push(X86Instruction::AllocateStack(aligned_bytes));
            }

            //fix up x86 hardware instruction constraints (using scratch registers)
            for inst in raw_instructions {
                match inst {
                    //memory-to-memory mov is illegal in x86
                    X86Instruction::Mov {
                        src: X86Operand::Stack(src_off),
                        dst: X86Operand::Stack(dst_off),
                    } => {
                        final_instructions.push(X86Instruction::Mov {
                            src: X86Operand::Stack(src_off),
                            dst: X86Operand::Reg(X86Reg::R10),
                        });
                        final_instructions.push(X86Instruction::Mov {
                            src: X86Operand::Reg(X86Reg::R10),
                            dst: X86Operand::Stack(dst_off),
                        });
                    }
                    //memory-to-memory add / sub is illegal
                    X86Instruction::Binary {
                        op: op @ (X86BinaryOp::Add | X86BinaryOp::Sub),
                        src: X86Operand::Stack(src_off),
                        dst: X86Operand::Stack(dst_off),
                    } => {
                        final_instructions.push(X86Instruction::Mov {
                            src: X86Operand::Stack(src_off),
                            dst: X86Operand::Reg(X86Reg::R10),
                        });
                        final_instructions.push(X86Instruction::Binary {
                            op,
                            src: X86Operand::Reg(X86Reg::R10),
                            dst: X86Operand::Stack(dst_off),
                        });
                    }
                    //imul destination cant be a memory address
                    X86Instruction::Binary {
                        op: X86BinaryOp::Mul,
                        src,
                        dst: X86Operand::Stack(dst_off),
                    } => {
                        //mov -dst(%rbp), %r11d
                        //imul src, %r11d
                        //mov %r11d, -dst(%rbp)
                        final_instructions.push(X86Instruction::Mov {
                            src: X86Operand::Stack(dst_off),
                            dst: X86Operand::Reg(X86Reg::R11),
                        });
                        final_instructions.push(X86Instruction::Binary {
                            op: X86BinaryOp::Mul,
                            src,
                            dst: X86Operand::Reg(X86Reg::R11),
                        });
                        final_instructions.push(X86Instruction::Mov {
                            src: X86Operand::Reg(X86Reg::R11),
                            dst: X86Operand::Stack(dst_off),
                        });
                    }
                    //idiv operand cannot be an immediate constant
                    X86Instruction::Idiv(X86Operand::Imm(val)) => {
                        final_instructions.push(X86Instruction::Mov {
                            src: X86Operand::Imm(val),
                            dst: X86Operand::Reg(X86Reg::R10),
                        });
                        final_instructions.push(X86Instruction::Idiv(X86Operand::Reg(X86Reg::R10)));
                    }
                    //cmpl memory-to-memory or second operand is immediate
                    X86Instruction::Cmp { src, dst } => {
                        match (src, dst) {
                            (X86Operand::Stack(src_off), X86Operand::Stack(dst_off)) => {
                                //movl -src(%rbp), %r10d
                                //cmpl %r10d, -dst(%rbp)
                                final_instructions.push(X86Instruction::Mov {
                                    src: X86Operand::Stack(src_off),
                                    dst: X86Operand::Reg(X86Reg::R10),
                                });
                                final_instructions.push(X86Instruction::Cmp {
                                    src: X86Operand::Reg(X86Reg::R10),
                                    dst: X86Operand::Stack(dst_off),
                                });
                            }
                            (src_op, X86Operand::Imm(val)) => {
                                //destination cannot be an immediate. Move to %r11d first.
                                final_instructions.push(X86Instruction::Mov {
                                    src: X86Operand::Imm(val),
                                    dst: X86Operand::Reg(X86Reg::R11),
                                });
                                final_instructions.push(X86Instruction::Cmp {
                                    src: src_op,
                                    dst: X86Operand::Reg(X86Reg::R11),
                                });
                            }
                            (src_op, dst_op) => {
                                final_instructions.push(X86Instruction::Cmp {
                                    src: src_op,
                                    dst: dst_op,
                                });
                            }
                        }
                    }
                    other => final_instructions.push(other),
                }
            }

            functions.push(X86Function {
                name: ir_fn.name.clone(),
                instructions: final_instructions,
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

            //function prologue
            writeln!(out, "    pushq %rbp").unwrap();
            writeln!(out, "    movq %rsp, %rbp").unwrap();

            for inst in &func.instructions {
                match inst {
                    X86Instruction::AllocateStack(bytes) => {
                        writeln!(out, "    subq ${bytes}, %rsp").unwrap();
                    }
                    X86Instruction::DeallocateStack => {
                        writeln!(out, "    movq %rbp, %rsp").unwrap();
                        writeln!(out, "    popq %rbp").unwrap();
                    }
                    X86Instruction::Mov { src, dst } => {
                        writeln!(
                            out,
                            "    movl {}, {}",
                            Self::format_operand(src),
                            Self::format_operand(dst)
                        )
                        .unwrap();
                    }
                    X86Instruction::Unary { op, dst } => {
                        let op_str = match op {
                            X86UnaryOp::Neg => "negl",
                            X86UnaryOp::Not => "notl",
                        };
                        writeln!(out, "    {} {}", op_str, Self::format_operand(dst)).unwrap();
                    }
                    X86Instruction::Cmp { src, dst } => {
                        writeln!(
                            out,
                            "    cmpl {}, {}",
                            Self::format_operand(src),
                            Self::format_operand(dst)
                        )
                        .unwrap();
                    }
                    X86Instruction::MovZeroExtend {
                        src_reg: X86Reg::AX,
                        dst,
                    } => {
                        writeln!(out, "    movzbl %al, {}", Self::format_operand(dst)).unwrap();
                    }
                    X86Instruction::Ret => {
                        writeln!(out, "    ret").unwrap();
                    }
                    X86Instruction::Binary { op, src, dst } => {
                        let op_str = match op {
                            X86BinaryOp::Add => "addl",
                            X86BinaryOp::Sub => "subl",
                            X86BinaryOp::Mul => "imull",
                        };
                        writeln!(
                            out,
                            "    {} {}, {}",
                            op_str,
                            Self::format_operand(src),
                            Self::format_operand(dst)
                        )
                        .unwrap();
                    }
                    X86Instruction::Cdq => {
                        writeln!(out, "    cdq").unwrap();
                    }
                    X86Instruction::Idiv(operand) => {
                        writeln!(out, "    idivl {}", Self::format_operand(operand)).unwrap();
                    }
                    X86Instruction::SetCc(cond, X86Reg::AX) => {
                        writeln!(out, "    set{} %al", cond).unwrap();
                    }
                    X86Instruction::Jmp(target) => {
                        writeln!(out, "    jmp {}", target).unwrap();
                    }
                    X86Instruction::JmpCc(cond, target) => {
                        writeln!(out, "    j{} {}", cond, target).unwrap();
                    }
                    X86Instruction::Label(label) => {
                        writeln!(out, "{}:", label).unwrap();
                    }
                    _ => unreachable!(),
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
            X86Operand::Reg(X86Reg::DX) => "%edx".to_string(),
            X86Operand::Reg(X86Reg::R10) => "%r10d".to_string(),
            X86Operand::Reg(X86Reg::R11) => "%r11d".to_string(),
            X86Operand::Stack(offset) => format!("{offset}(%rbp)"),
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
                X86Instruction::DeallocateStack,
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
            "    pushq %rbp\n",
            "    movq %rsp, %rbp\n",
            "    movl $42, %eax\n",
            "    movq %rbp, %rsp\n",
            "    popq %rbp\n",
            "    ret\n",
            "    .section .note.GNU-stack,\"\",@progbits\n"
        );
        assert_eq!(asm, expected);
    }

    #[test]
    fn lowers_unary_negation_and_complement() {
        let src = "int main(void) { return ~(-42); }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);
        let x86 = Codegen::generate_x86(&ir);

        //should contain Neg, Not, and return in EAX
        let has_neg = x86.functions[0].instructions.iter().any(|i| {
            matches!(
                i,
                X86Instruction::Unary {
                    op: X86UnaryOp::Neg,
                    ..
                }
            )
        });
        let has_not = x86.functions[0].instructions.iter().any(|i| {
            matches!(
                i,
                X86Instruction::Unary {
                    op: X86UnaryOp::Not,
                    ..
                }
            )
        });
        assert!(has_neg);
        assert!(has_not);
    }

    #[test]
    fn lowers_logical_not_to_cmp_sete_movzbl() {
        let src = "int main(void) { return !0; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);
        let x86 = Codegen::generate_x86(&ir);

        let has_cmp = x86.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, X86Instruction::Cmp { .. }));
        let has_sete = x86.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, X86Instruction::SetCc(X86CondCode::E, X86Reg::AX)));
        let has_movzbl = x86.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, X86Instruction::MovZeroExtend { .. }));

        assert!(has_cmp);
        assert!(has_sete);
        assert!(has_movzbl);
    }

    #[test]
    fn lowers_addition_subtraction_multiplication() {
        let src = "int main(void) { return 1 + 2 * 3 - 4; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);
        let x86 = Codegen::generate_x86(&ir);

        let has_add = x86.functions[0].instructions.iter().any(|i| {
            matches!(
                i,
                X86Instruction::Binary {
                    op: X86BinaryOp::Add,
                    ..
                }
            )
        });
        let has_sub = x86.functions[0].instructions.iter().any(|i| {
            matches!(
                i,
                X86Instruction::Binary {
                    op: X86BinaryOp::Sub,
                    ..
                }
            )
        });
        let has_mul = x86.functions[0].instructions.iter().any(|i| {
            matches!(
                i,
                X86Instruction::Binary {
                    op: X86BinaryOp::Mul,
                    ..
                }
            )
        });

        assert!(has_add);
        assert!(has_sub);
        assert!(has_mul);
    }

    #[test]
    fn lowers_division_and_modulo_with_cdq_and_idiv() {
        let src = "int main(void) { return 10 / 2 + 7 % 3; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);
        let x86 = Codegen::generate_x86(&ir);

        let has_cdq = x86.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, X86Instruction::Cdq));
        let has_idiv = x86.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, X86Instruction::Idiv(_)));

        assert!(has_cdq);
        assert!(has_idiv);
    }

    #[test]
    fn generates_code_for_relational_comparison() {
        let src = "int main(void) { return 3 <= 5; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);
        let x86 = Codegen::generate_x86(&ir);

        let has_set_cc = x86.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, X86Instruction::SetCc(X86CondCode::Le, _)));
        assert!(has_set_cc);
    }

    #[test]
    fn generates_code_for_short_circuit_control_flow() {
        let src = "int main(void) { return 0 && 5; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);
        let x86 = Codegen::generate_x86(&ir);

        let has_jmp_cc = x86.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, X86Instruction::JmpCc(X86CondCode::E, _)));
        let has_label = x86.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, X86Instruction::Label(_)));
        assert!(has_jmp_cc);
        assert!(has_label);
    }
}
