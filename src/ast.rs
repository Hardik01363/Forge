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

use std::ops::Index;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AstArena {
    nodes: Vec<ExprNode>,
}

impl AstArena {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    //allocates an expression node into the arena and returns its unique ID.
    pub fn alloc(&mut self, kind: ExprKind, span: Span) -> ExprId {
        let index = u32::try_from(self.nodes.len()).expect("AstArena overflowed u32::MAX nodes");
        self.nodes.push(ExprNode { kind, span });
        ExprId(index)
    }

    //returns the total number of allocated expression nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    //retrieves a node reference by its ID.
    pub fn get(&self, id: ExprId) -> Option<&ExprNode> {
        self.nodes.get(id.0 as usize)
    }
}

impl Index<ExprId> for AstArena {
    type Output = ExprNode;

    fn index(&self, id: ExprId) -> &Self::Output {
        &self.nodes[id.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_allocates_and_indexes_nodes() {
        let mut arena = AstArena::new();
        let span = Span { start: 0, end: 2 };
        let id1 = arena.alloc(ExprKind::Integer(42), span);
        let id2 = arena.alloc(
            ExprKind::Unary {
                op: UnaryOp::Negate,
                operand: id1,
            },
            span,
        );

        assert_eq!(id1, ExprId(0));
        assert_eq!(id2, ExprId(1));
        assert_eq!(arena[id1].kind, ExprKind::Integer(42));
        assert_eq!(
            arena[id2].kind,
            ExprKind::Unary {
                op: UnaryOp::Negate,
                operand: id1,
            }
        );
    }
}
