#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntSuffix {
    Unsuffixed,
    U,
    L,
    UL,
    LL,
    ULL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntBase {
    Decimal,
    Octal,
    Hex,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSuffix {
    Unsuffixed,
    F,
    L,
}

#[derive(Debug, Clone, PartialEq)]

pub enum TokenKind {
    //keywords
    Int,
    Void,
    Return,

    //names and literals (carry data)
    Identifier(String),
    Constant {
        value: u64,
        suffix: IntSuffix,
        base: IntBase,
    },
    FloatConstant {
        value: f64,
        suffix: FloatSuffix,
    },

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

#[derive(Debug, Clone, PartialEq)]
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

    fn parse_int_suffix(s: &str) -> Result<IntSuffix, ()> {
        match s {
            "" => Ok(IntSuffix::Unsuffixed),
            "u" | "U" => Ok(IntSuffix::U),
            "l" | "L" => Ok(IntSuffix::L),
            "ll" | "LL" => Ok(IntSuffix::LL),
            "ul" | "uL" | "Ul" | "UL" | "lu" | "Lu" | "lU" | "LU" => Ok(IntSuffix::UL),
            "ull" | "uLL" | "Ull" | "ULL" | "llu" | "llU" | "LLu" | "LLU" => Ok(IntSuffix::ULL),
            _ => Err(()),
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
            //identifiers cant start with a digit, but, can contain a digit later on in the name.
            //Thus, this match branch doesnt trigger when the first char encountered is a digit,
            //but, accepts digits afterwards in the inner while loop condition check
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                while let Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') = self.peek() {
                    self.advance();
                }
                match &self.source[start..self.pos] {
                    "int" => TokenKind::Int,
                    "void" => TokenKind::Void,
                    "return" => TokenKind::Return,
                    text => TokenKind::Identifier(text.to_string()),
                }
            }
            //starts with 1-9 as, in C, leading 0 means octal base number.
            b'1'..=b'9' => {
                while let Some(b'0'..=b'9') = self.peek() {
                    self.advance();
                }
                let digits_end = self.pos;
                while let Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') = self.peek() {
                    self.advance();
                }
                let suffix_str = &self.source[digits_end..self.pos];
                let suffix = match Self::parse_int_suffix(suffix_str) {
                    Ok(s) => s,
                    Err(_) => {
                        return Some(Err(LexError {
                            message: "invalid suffix on integer constant".to_string(),
                            span: Span {
                                start,
                                end: self.pos,
                            },
                        }));
                    }
                };
                match self.source[start..digits_end].parse::<u64>() {
                    Ok(value) => TokenKind::Constant {
                        value,
                        suffix,
                        base: IntBase::Decimal,
                    },
                    Err(_) => {
                        return Some(Err(LexError {
                            message: "integer constant is too large".to_string(),
                            span: Span {
                                start,
                                end: self.pos,
                            },
                        }));
                    }
                }
            }
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

    #[test]
    fn identifier_is_one_token() {
        let mut lexer = Lexer::new("_foo_1 ");
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(token.kind, TokenKind::Identifier("_foo_1".to_string()));
        assert_eq!(token.span, Span { start: 0, end: 6 });
    }

    #[test]
    fn keywords_are_recognized() {
        let kinds: Vec<TokenKind> = Lexer::new("int void return")
            .map(|item| item.unwrap().kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Int,
                TokenKind::Void,
                TokenKind::Return,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keyword_lookalikes_stay_identifiers() {
        let kinds: Vec<TokenKind> = Lexer::new("integer returned int2")
            .map(|item| item.unwrap().kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier("integer".to_string()),
                TokenKind::Identifier("returned".to_string()),
                TokenKind::Identifier("int2".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn decimal_constant_has_value_base_and_span() {
        let token = Lexer::new("42;").next().unwrap().unwrap();
        assert_eq!(
            token.kind,
            TokenKind::Constant {
                value: 42,
                suffix: IntSuffix::Unsuffixed,
                base: IntBase::Decimal,
            }
        );
        assert_eq!(token.span, Span { start: 0, end: 2 });
    }

    #[test]
    fn largest_u64_is_accepted() {
        let token = Lexer::new("18446744073709551615").next().unwrap().unwrap();
        assert_eq!(
            token.kind,
            TokenKind::Constant {
                value: u64::MAX,
                suffix: IntSuffix::Unsuffixed,
                base: IntBase::Decimal,
            }
        );
    }

    #[test]
    fn constant_too_large_is_an_error() {
        let error = Lexer::new("18446744073709551616")
            .next()
            .unwrap()
            .unwrap_err();
        assert_eq!(error.span, Span { start: 0, end: 20 });
    }

    #[test]
    fn lexer_recovers_after_integer_overflow() {
        let mut lexer = Lexer::new("99999999999999999999 7");

        let err = lexer.next().unwrap().unwrap_err();
        assert_eq!(err.message, "integer constant is too large");
        assert_eq!(err.span, Span { start: 0, end: 20 });

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(
            token.kind,
            TokenKind::Constant {
                value: 7,
                suffix: IntSuffix::Unsuffixed,
                base: IntBase::Decimal,
            }
        );
        assert_eq!(token.span, Span { start: 21, end: 22 });
    }

    #[test]
    fn integer_suffixes_are_parsed() {
        let cases = [
            ("42u", IntSuffix::U),
            ("42U", IntSuffix::U),
            ("42l", IntSuffix::L),
            ("42L", IntSuffix::L),
            ("42ll", IntSuffix::LL),
            ("42LL", IntSuffix::LL),
            ("42ul", IntSuffix::UL),
            ("42LU", IntSuffix::UL),
            ("42ull", IntSuffix::ULL),
            ("42LLU", IntSuffix::ULL),
        ];

        for (src, expected_suffix) in cases {
            let token = Lexer::new(src).next().unwrap().unwrap();
            assert_eq!(
                token.kind,
                TokenKind::Constant {
                    value: 42,
                    suffix: expected_suffix,
                    base: IntBase::Decimal,
                }
            );
            assert_eq!(
                token.span,
                Span {
                    start: 0,
                    end: src.len()
                }
            );
        }
    }

    #[test]
    fn invalid_integer_suffix_is_rejected() {
        let invalid = ["42lL", "42Ll", "42ulL", "42z", "1foo"];
        for src in invalid {
            let err = Lexer::new(src).next().unwrap().unwrap_err();
            assert_eq!(err.message, "invalid suffix on integer constant");
            assert_eq!(
                err.span,
                Span {
                    start: 0,
                    end: src.len()
                }
            );
        }
    }
}
