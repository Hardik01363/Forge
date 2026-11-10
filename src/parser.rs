use crate::ast::{AstArena, BinaryOp, ExprId, ExprKind, UnaryOp};
use crate::lexer::{LexError, Lexer, Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl From<LexError> for ParseError {
    fn from(err: LexError) -> Self {
        Self {
            message: err.message,
            span: err.span,
        }
    }
}

//returns the prefix binding power for unary operators.
fn prefix_binding_power(op: &TokenKind) -> Option<((), u8)> {
    match op {
        TokenKind::Minus | TokenKind::Tilde | TokenKind::Bang => Some(((), 9)),
        _ => None,
    }
}

//returns (left_bp, right_bp) for infix binary operators.
fn infix_binding_power(op: &TokenKind) -> Option<(u8, u8)> {
    match op {
        TokenKind::Plus | TokenKind::Minus => Some((5, 6)),
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some((7, 8)),
        _ => None,
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    arena: AstArena,
    peek_token: Option<Result<Token, LexError>>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut lexer = Lexer::new(source);
        let peek_token = lexer.next();
        Self {
            lexer,
            arena: AstArena::new(),
            peek_token,
        }
    }

    //returns a reference to the peeked token kind, or EOF if exhausted.
    fn peek_kind(&self) -> Result<&TokenKind, ParseError> {
        match &self.peek_token {
            Some(Ok(token)) => Ok(&token.kind),
            Some(Err(err)) => Err(ParseError::from(err.clone())),
            None => Ok(&TokenKind::Eof),
        }
    }

    //returns the span of the peeked token.
    fn peek_span(&self) -> Span {
        match &self.peek_token {
            Some(Ok(token)) => token.span,
            Some(Err(err)) => err.span,
            None => Span { start: 0, end: 0 },
        }
    }

    //consumes the current token and advances to the next.
    fn advance(&mut self) -> Result<Token, ParseError> {
        let current = self.peek_token.take();
        self.peek_token = self.lexer.next();
        match current {
            Some(Ok(token)) => Ok(token),
            Some(Err(err)) => Err(ParseError::from(err)),
            None => Ok(Token {
                kind: TokenKind::Eof,
                span: Span { start: 0, end: 0 },
            }),
        }
    }

    //expects the current token to match `expected`, consumes it, or returns an error.
    fn expect(&mut self, expected: &TokenKind) -> Result<Token, ParseError> {
        let span = self.peek_span();
        let token = self.advance()?;
        if &token.kind == expected {
            Ok(token)
        } else {
            Err(ParseError {
                message: format!("expected {expected:?}, found {:?}", token.kind),
                span,
            })
        }
    }

    //convenience entry point to parse a complete expression from source.
    pub fn parse_all(source: &'a str) -> Result<(ExprId, AstArena), ParseError> {
        let mut parser = Self::new(source);
        let root = parser.parse_expression()?;
        parser.expect(&TokenKind::Eof)?;
        Ok((root, parser.arena))
    }

    //parses an expression starting with default minimum binding power (0).
    pub fn parse_expression(&mut self) -> Result<ExprId, ParseError> {
        self.parse_expr_bp(0)
    }

    //the Pratt Parsing loop (parses expressions where operators have at least `min_bp`)
    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<ExprId, ParseError> {
        let token = self.advance()?;
        let mut lhs = match token.kind {
            TokenKind::Constant { value, .. } => {
                self.arena.alloc(ExprKind::Integer(value), token.span)
            }
            TokenKind::FloatConstant { value, .. } => {
                self.arena.alloc(ExprKind::Float(value), token.span)
            }
            TokenKind::Identifier(name) => self.arena.alloc(ExprKind::Identifier(name), token.span),
            TokenKind::OpenParen => {
                let inner = self.parse_expression()?;
                let close = self.expect(&TokenKind::CloseParen)?;
                let span = Span {
                    start: token.span.start,
                    end: close.span.end,
                };
                self.arena.alloc(ExprKind::Grouping(inner), span)
            }
            op => {
                if let Some(((), r_bp)) = prefix_binding_power(&op) {
                    let rhs = self.parse_expr_bp(r_bp)?;
                    let unary_op = match op {
                        TokenKind::Minus => UnaryOp::Negate,
                        TokenKind::Tilde => UnaryOp::Complement,
                        TokenKind::Bang => UnaryOp::Not,
                        _ => unreachable!(),
                    };
                    let rhs_span = self.arena[rhs].span;
                    let span = Span {
                        start: token.span.start,
                        end: rhs_span.end,
                    };
                    self.arena.alloc(
                        ExprKind::Unary {
                            op: unary_op,
                            operand: rhs,
                        },
                        span,
                    )
                } else {
                    return Err(ParseError {
                        message: format!("unexpected token in expression: {op:?}"),
                        span: token.span,
                    });
                }
            }
        };

        loop {
            let next_op = self.peek_kind()?.clone();
            if let Some((l_bp, r_bp)) = infix_binding_power(&next_op) {
                if l_bp < min_bp {
                    break;
                }
                let op_token = self.advance()?;
                let binary_op = match op_token.kind {
                    TokenKind::Plus => BinaryOp::Add,
                    TokenKind::Minus => BinaryOp::Subtract,
                    TokenKind::Star => BinaryOp::Multiply,
                    TokenKind::Slash => BinaryOp::Divide,
                    TokenKind::Percent => BinaryOp::Modulo,
                    _ => unreachable!(),
                };
                let rhs = self.parse_expr_bp(r_bp)?;
                let lhs_span = self.arena[lhs].span;
                let rhs_span = self.arena[rhs].span;
                let span = Span {
                    start: lhs_span.start,
                    end: rhs_span.end,
                };
                lhs = self.arena.alloc(
                    ExprKind::Binary {
                        op: binary_op,
                        left: lhs,
                        right: rhs,
                    },
                    span,
                );
                continue;
            }
            break;
        }

        Ok(lhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_integer_literal() {
        let (root, arena) = Parser::parse_all("42").unwrap();
        assert_eq!(arena[root].kind, ExprKind::Integer(42));
    }

    #[test]
    fn parses_unary_prefix_expression() {
        let (root, arena) = Parser::parse_all("-42").unwrap();
        match &arena[root].kind {
            ExprKind::Unary { op, operand } => {
                assert_eq!(*op, UnaryOp::Negate);
                assert_eq!(arena[*operand].kind, ExprKind::Integer(42));
            }
            other => panic!("expected Unary, got {other:?}"),
        }
    }

    #[test]
    fn binary_precedence_multiplication_binds_tighter_than_addition() {
        //1 + 2 * 3 should parse as 1 + (2 * 3)
        let (root, arena) = Parser::parse_all("1 + 2 * 3").unwrap();
        match &arena[root].kind {
            ExprKind::Binary { op, left, right } => {
                assert_eq!(*op, BinaryOp::Add);
                assert_eq!(arena[*left].kind, ExprKind::Integer(1));
                //right must be 2 * 3
                match &arena[*right].kind {
                    ExprKind::Binary { op, left, right } => {
                        assert_eq!(*op, BinaryOp::Multiply);
                        assert_eq!(arena[*left].kind, ExprKind::Integer(2));
                        assert_eq!(arena[*right].kind, ExprKind::Integer(3));
                    }
                    other => panic!("expected right hand side to be Multiply, got {other:?}"),
                }
            }
            other => panic!("expected Binary Add, got {other:?}"),
        }
    }

    #[test]
    fn binary_left_associative_subtraction() {
        //1 - 2 - 3 should parse as (1 - 2) - 3
        let (root, arena) = Parser::parse_all("1 - 2 - 3").unwrap();
        match &arena[root].kind {
            ExprKind::Binary { op, left, right } => {
                assert_eq!(*op, BinaryOp::Subtract);
                assert_eq!(arena[*right].kind, ExprKind::Integer(3));
                //left must be 1 - 2
                match &arena[*left].kind {
                    ExprKind::Binary { op, left, right } => {
                        assert_eq!(*op, BinaryOp::Subtract);
                        assert_eq!(arena[*left].kind, ExprKind::Integer(1));
                        assert_eq!(arena[*right].kind, ExprKind::Integer(2));
                    }
                    other => panic!("expected left hand side to be Subtract, got {other:?}"),
                }
            }
            other => panic!("expected Binary Subtract, got {other:?}"),
        }
    }

    #[test]
    fn parenthesized_expression_overrides_precedence() {
        //(1 + 2) * 3
        let (root, arena) = Parser::parse_all("(1 + 2) * 3").unwrap();
        match &arena[root].kind {
            ExprKind::Binary { op, left, right } => {
                assert_eq!(*op, BinaryOp::Multiply);
                assert_eq!(arena[*right].kind, ExprKind::Integer(3));
                match &arena[*left].kind {
                    ExprKind::Grouping(inner) => match &arena[*inner].kind {
                        ExprKind::Binary { op, left, right } => {
                            assert_eq!(*op, BinaryOp::Add);
                            assert_eq!(arena[*left].kind, ExprKind::Integer(1));
                            assert_eq!(arena[*right].kind, ExprKind::Integer(2));
                        }
                        other => panic!("expected Add to be inside grouping, got {other:?}"),
                    },
                    other => panic!("expected Grouping, got {other:?}"),
                }
            }
            other => panic!("expected Multiply, got {other:?}"),
        }
    }
}
