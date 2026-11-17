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

pub struct IrLowerer<'a> {
    arena: &'a AstArena,
    instructions: Vec<Instruction>,
    var_counter: usize,
}

impl<'a> IrLowerer<'a> {
    pub fn new(arena: &'a AstArena) -> Self {
        Self {
            arena,
            instructions: Vec::new(),
            var_counter: 0,
        }
    }

    //generates a fresh, unique temporary variable name.
    fn make_temporary(&mut self) -> Val {
        let name = format!("tmp.{}", self.var_counter);
        self.var_counter += 1;
        Val::Var(name)
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
            ExprKind::Float(_) => todo!("floating point lowering in Stage 2 Part II"),
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
