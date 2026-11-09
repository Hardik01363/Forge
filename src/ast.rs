use crate::lexer::Span;

//strongly-typed 32-bit identifier pointing into the AST arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,     // -
    Complement, // ~
    Not,        // !
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Integer(u64),
    Float(f64),
    Identifier(String),
    Unary {
        op: UnaryOp,
        operand: ExprId,
    },
    Binary {
        op: BinaryOp,
        left: ExprId,
        right: ExprId,
    },
    Grouping(ExprId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprNode {
    pub kind: ExprKind,
    pub span: Span,
}
