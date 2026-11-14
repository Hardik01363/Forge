use crate::lexer::Span;
use std::fmt;

//strongly-typed 32-bit identifier pointing into the AST arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,     // -
    Complement, // ~
    Not,        // !
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Negate => write!(f, "-"),
            UnaryOp::Complement => write!(f, "~"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Subtract => write!(f, "-"),
            BinaryOp::Multiply => write!(f, "*"),
            BinaryOp::Divide => write!(f, "/"),
            BinaryOp::Modulo => write!(f, "%"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Return(ExprId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub name: String,
    pub body: Statement,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub function: FunctionDeclaration,
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

    //dumps an expression tree rooted at `root` into an indented, human readable string (i used
    //similar format to what clang uses, generates 2 spaces of indentation per depth level)
    pub fn dump_tree(&self, root: ExprId) -> String {
        let mut output = String::new();
        self.format_node(root, 0, &mut output);
        output
    }

    fn format_node(&self, id: ExprId, indent: usize, out: &mut String) {
        let pad = "  ".repeat(indent);
        let node = &self[id];
        match &node.kind {
            ExprKind::Integer(val) => {
                out.push_str(&format!("{pad}IntegerLiteral {val}\n"));
            }
            ExprKind::Float(val) => {
                out.push_str(&format!("{pad}FloatingLiteral {val}\n"));
            }
            ExprKind::Identifier(name) => {
                out.push_str(&format!("{pad}DeclRefExpr '{name}'\n"));
            }
            ExprKind::Unary { op, operand } => {
                out.push_str(&format!("{pad}UnaryOperator '{op}'\n"));
                self.format_node(*operand, indent + 1, out);
            }
            ExprKind::Binary { op, left, right } => {
                out.push_str(&format!("{pad}BinaryOperator '{op}'\n"));
                self.format_node(*left, indent + 1, out);
                self.format_node(*right, indent + 1, out);
            }
            ExprKind::Grouping(inner) => {
                out.push_str(&format!("{pad}ParenExpr\n"));
                self.format_node(*inner, indent + 1, out);
            }
        }
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
