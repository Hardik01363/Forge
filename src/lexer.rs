#[derive(Debug, Clone, PartialEq, Eq)]

pub enum TokenKind {
    //keywords
    Int,
    Main,
    Return,

    //names and literals (carry data)
    Identifier(String),
    Constant(u64),

    //punctuation
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Semicolon,
    EoF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

pub struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    finished: bool, //required field so as to classify EoF character
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source,
            pos: 0,
            finished: false, //flipped when we hit EoF
        }
    }

    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let byte = self.peek()?; //? ensures that we return immediately if none is encountered, so
                                 //that pos never moves past the end
        self.pos += 1;
        Some(byte)
    }

    fn skip_whitespace(&mut self) {
        // we accept 6 kinds of bytes as whitespace, as defined in the C source code (from
        // cppreference.com)
        while let Some(b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) = self.peek() {
            self.advance();
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        self.skip_whitespace();

        if self.peek().is_none() {
            self.finished = true;
            let span = Span {
                start: self.pos,
                end: self.pos,
            };
            return Some(Ok(Token {
                kind: TokenKind::Eof,
                span,
            }));
        }

        todo!("lex real (non-whitespace/EoF) tokens")
    }
}
