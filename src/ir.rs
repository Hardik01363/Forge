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
    Equal,
    NotEqual,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
}

impl From<BinaryOp> for IrBinaryOp {
    fn from(op: BinaryOp) -> Self {
        match op {
            BinaryOp::Add => IrBinaryOp::Add,
            BinaryOp::Subtract => IrBinaryOp::Subtract,
            BinaryOp::Multiply => IrBinaryOp::Multiply,
            BinaryOp::Divide => IrBinaryOp::Divide,
            BinaryOp::Modulo => IrBinaryOp::Modulo,
            BinaryOp::Equal => IrBinaryOp::Equal,
            BinaryOp::NotEqual => IrBinaryOp::NotEqual,
            BinaryOp::LessThan => IrBinaryOp::LessThan,
            BinaryOp::LessOrEqual => IrBinaryOp::LessOrEqual,
            BinaryOp::GreaterThan => IrBinaryOp::GreaterThan,
            BinaryOp::GreaterOrEqual => IrBinaryOp::GreaterOrEqual,
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                panic!("LogicalAnd and LogicalOr should be lowered with control flow, not as IrBinaryOp")
            }
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
    Label(String),
    Jump(String),
    JumpIfZero {
        condition: Val,
        target: String,
    },
    JumpIfNotZero {
        condition: Val,
        target: String,
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

pub struct IrLowerer<'a> {
    arena: &'a AstArena,
    instructions: Vec<Instruction>,
    var_counter: usize,
    label_counter: usize,
}

impl<'a> IrLowerer<'a> {
    pub fn new(arena: &'a AstArena) -> Self {
        Self {
            arena,
            instructions: Vec::new(),
            var_counter: 0,
            label_counter: 0,
        }
    }

    //generates a fresh, unique temporary variable name.
    fn make_temporary(&mut self) -> Val {
        let name = format!("tmp.{}", self.var_counter);
        self.var_counter += 1;
        Val::Var(name)
    }

    //generates a fresh, unique temporary label name.
    fn make_label(&mut self, prefix: &str) -> String {
        let label = format!(".L{}.{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    //lowers an AST expression into a sequence of TAC instructions,
    //returning the `Val` (constant or temporary) that holds the final result.
    pub fn lower_expr(&mut self, expr_id: ExprId) -> Val {
        let node = &self.arena[expr_id];
        match &node.kind {
            ExprKind::Integer(val) => Val::Constant(*val as i64),
            ExprKind::Identifier(name) => Val::Var(name.clone()),
            ExprKind::Grouping(inner) => self.lower_expr(*inner),
            ExprKind::Unary { op, operand } => {
                let src = self.lower_expr(*operand);
                let dst = self.make_temporary();
                self.instructions.push(Instruction::Unary {
                    op: IrUnaryOp::from(*op),
                    src,
                    dst: dst.clone(),
                });
                dst
            }
            ExprKind::Binary { op, left, right } => {
                match op {
                    BinaryOp::LogicalAnd => {
                        //short-circuit steps for AND:
                        //evaluate left
                        //if left == 0, jump to false_label
                        //evaluate right
                        //if right == 0, jump to false_label
                        //dst = 1; jump to end_label
                        //false_label:
                        //dst = 0
                        //end_label:
                        let dst = self.make_temporary();
                        let false_label = self.make_label("and_false");
                        let end_label = self.make_label("and_end");

                        let left_val = self.lower_expr(*left);
                        self.instructions.push(Instruction::JumpIfZero {
                            condition: left_val,
                            target: false_label.clone(),
                        });

                        let right_val = self.lower_expr(*right);
                        self.instructions.push(Instruction::JumpIfZero {
                            condition: right_val,
                            target: false_label.clone(),
                        });

                        self.instructions.push(Instruction::Copy {
                            src: Val::Constant(1),
                            dst: dst.clone(),
                        });
                        self.instructions.push(Instruction::Jump(end_label.clone()));

                        self.instructions.push(Instruction::Label(false_label));
                        self.instructions.push(Instruction::Copy {
                            src: Val::Constant(0),
                            dst: dst.clone(),
                        });

                        self.instructions.push(Instruction::Label(end_label));
                        dst
                    }
                    BinaryOp::LogicalOr => {
                        //short-circuit steps for OR:
                        //evaluate left
                        //if left != 0, jump to true_label
                        //evaluate right
                        //if right != 0, jump to true_label
                        //dst = 0; jump to end_label
                        //true_label:
                        //dst = 1
                        //end_label:
                        let dst = self.make_temporary();
                        let true_label = self.make_label("or_true");
                        let end_label = self.make_label("or_end");

                        let left_val = self.lower_expr(*left);
                        self.instructions.push(Instruction::JumpIfNotZero {
                            condition: left_val,
                            target: true_label.clone(),
                        });

                        let right_val = self.lower_expr(*right);
                        self.instructions.push(Instruction::JumpIfNotZero {
                            condition: right_val,
                            target: true_label.clone(),
                        });

                        self.instructions.push(Instruction::Copy {
                            src: Val::Constant(0),
                            dst: dst.clone(),
                        });
                        self.instructions.push(Instruction::Jump(end_label.clone()));

                        self.instructions.push(Instruction::Label(true_label));
                        self.instructions.push(Instruction::Copy {
                            src: Val::Constant(1),
                            dst: dst.clone(),
                        });

                        self.instructions.push(Instruction::Label(end_label));
                        dst
                    }
                    _ => {
                        let left_val = self.lower_expr(*left);
                        let right_val = self.lower_expr(*right);
                        let dst = self.make_temporary();
                        self.instructions.push(Instruction::Binary {
                            op: IrBinaryOp::from(*op),
                            left: left_val,
                            right: right_val,
                            dst: dst.clone(),
                        });
                        dst
                    }
                }
            }
            ExprKind::Float(_) => todo!("will implement floating point lowering later :D"),
        }
    }

    //lowers an AST program into an `IrProgram`.
    pub fn lower_program(arena: &AstArena, program: &Program) -> IrProgram {
        let mut lowerer = IrLowerer::new(arena);

        match &program.function.body {
            Statement::Return(expr_id) => {
                let result_val = lowerer.lower_expr(*expr_id);
                lowerer.instructions.push(Instruction::Return(result_val));
            }
        }

        let function = IrFunction {
            name: program.function.name.clone(),
            instructions: lowerer.instructions,
        };

        IrProgram {
            functions: vec![function],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn lowers_simple_return_constant() {
        let src = "int main(void) { return 42; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);

        assert_eq!(ir.functions.len(), 1);
        let f = &ir.functions[0];
        assert_eq!(f.name, "main");
        assert_eq!(f.instructions, vec![Instruction::Return(Val::Constant(42))]);
    }

    #[test]
    fn lowers_complex_nested_expression_into_three_address_code() {
        //return 1 + 2 * 3;
        //In TAC, this must linearize into the below 3 instructions:
        //tmp.0 = 2 * 3
        //tmp.1 = 1 + tmp.0
        //return tmp.1
        let src = "int main(void) { return 1 + 2 * 3; }";
        let parsed = Parser::parse_program(src).unwrap();
        let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);

        let f = &ir.functions[0];
        assert_eq!(
            f.instructions,
            vec![
                Instruction::Binary {
                    op: IrBinaryOp::Multiply,
                    left: Val::Constant(2),
                    right: Val::Constant(3),
                    dst: Val::Var("tmp.0".to_string()),
                },
                Instruction::Binary {
                    op: IrBinaryOp::Add,
                    left: Val::Constant(1),
                    right: Val::Var("tmp.0".to_string()),
                    dst: Val::Var("tmp.1".to_string()),
                },
                Instruction::Return(Val::Var("tmp.1".to_string())),
            ]
        );
    }
}
