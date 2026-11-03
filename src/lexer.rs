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
    Eof,
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

        let start = self.pos;
        let byte = self.advance()?;
        let kind = match byte {
            b'(' => TokenKind::OpenParen,
            b')' => TokenKind::CloseParen,
            b'{' => TokenKind::OpenBrace,
            b'}' => TokenKind::CloseBrace,
            b';' => TokenKind::Semicolon,
            _ => todo!("lex identifiers, constants, errors"),
        };
        Some(Ok(Token {
            kind,
            span: Span {
                start,
                end: self.pos,
            },
        }))
    }
}

//the following tests are compiled only for cargo test cmd. they are added here so that i can check
//if intended behaviour is followed whenever i change something.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_gives_only_eof() {
        let items: Vec<_> = Lexer::new("").collect();
        let eof = Token {
            kind: TokenKind::Eof,
            span: Span { start: 0, end: 0 },
        };
        assert_eq!(items, vec![Ok(eof)]);
    }

    #[test]
    fn whitespace_is_skipped_before_eof() {
        let items: Vec<_> = Lexer::new("  \n\t ").collect();
        let eof = Token {
            kind: TokenKind::Eof,
            span: Span { start: 5, end: 5 },
        };
        assert_eq!(items, vec![Ok(eof)]);
    }

    #[test]
    fn iterator_ends_after_eof() {
        let mut lexer = Lexer::new("");
        assert!(lexer.next().is_some());
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn open_paren_is_a_token() {
        let mut lexer = Lexer::new("(");
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(token.kind, TokenKind::OpenParen);
        assert_eq!(token.span, Span { start: 0, end: 1 });
    }

    #[test]
    fn punctuation_sequence_is_lexed_in_order() {
        let kinds: Vec<TokenKind> = Lexer::new("{};").map(|item| item.unwrap().kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::OpenBrace,
                TokenKind::CloseBrace,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn eof_follows_the_last_punctuation_token() {
        let tokens: Vec<Token> = Lexer::new("(){}").map(|item| item.unwrap()).collect();
        assert_eq!(tokens.len(), 5);
        let last = tokens.last().unwrap();
        assert_eq!(last.kind, TokenKind::Eof);
        assert_eq!(last.span, Span { start: 4, end: 4 });
    }
}
