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
}
