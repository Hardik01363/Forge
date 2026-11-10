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

    //punctuation and operators
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Tilde, //for bitwise NOT
    Bang,  //for logical NOT
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
        //we accept 6 kinds of bytes as whitespace, as defined in the C source code (from
        //cppreference.com)
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

    fn finish_int_constant(
        &mut self,
        start: usize,
        digits_start: usize,
        radix: u32,
        base: IntBase,
        has_separator: bool,
    ) -> Option<Result<Token, LexError>> {
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

        let raw_digits = &self.source[digits_start..digits_end];
        let parsed = if has_separator {
            let filtered: String = raw_digits.chars().filter(|&c| c != '\'').collect();
            u64::from_str_radix(&filtered, radix)
        } else {
            u64::from_str_radix(raw_digits, radix)
        };

        match parsed {
            Ok(value) => Some(Ok(Token {
                kind: TokenKind::Constant {
                    value,
                    suffix,
                    base,
                },
                span: Span {
                    start,
                    end: self.pos,
                },
            })),
            Err(_) => Some(Err(LexError {
                message: "integer constant is too large".to_string(),
                span: Span {
                    start,
                    end: self.pos,
                },
            })),
        }
    }

    fn consume_digits_with_separators(
        &mut self,
        is_digit: fn(u8) -> bool,
        allow_leading_sep: bool,
    ) -> Result<bool, &'static str> {
        let mut has_separator = false;
        let mut prev_was_sep = false;
        let start_pos = self.pos;

        while let Some(b) = self.peek() {
            if is_digit(b) {
                self.advance();
                prev_was_sep = false;
            } else if b == b'\'' {
                if prev_was_sep {
                    return Err("consecutive digit separators are invalid");
                }
                if self.pos == start_pos && !allow_leading_sep {
                    return Err("digit separator cannot appear immediately after prefix");
                }
                self.advance();
                has_separator = true;
                prev_was_sep = true;
            } else {
                break;
            }
        }

        if prev_was_sep {
            return Err("digit separator cannot appear at end of number");
        }

        Ok(has_separator)
    }

    fn parse_float_suffix(s: &str) -> Result<FloatSuffix, ()> {
        match s {
            "" => Ok(FloatSuffix::Unsuffixed),
            "f" | "F" => Ok(FloatSuffix::F),
            "l" | "L" => Ok(FloatSuffix::L),
            _ => Err(()),
        }
    }

    fn finish_float_constant(&mut self, start: usize) -> Option<Result<Token, LexError>> {
        //if peek is '.', consume it and the fractional digits
        if self.peek() == Some(b'.') {
            self.advance();
            while let Some(b'0'..=b'9' | b'\'') = self.peek() {
                self.advance();
            }
        } else {
            //already started with '.', just consume fractional digits
            while let Some(b'0'..=b'9' | b'\'') = self.peek() {
                self.advance();
            }
        }

        //exponent 'e' or 'E'
        if let Some(b'e' | b'E') = self.peek() {
            self.advance();
            if let Some(b'+' | b'-') = self.peek() {
                self.advance();
            }
            let exp_start = self.pos;
            while let Some(b'0'..=b'9' | b'\'') = self.peek() {
                self.advance();
            }
            if self.pos == exp_start {
                return Some(Err(LexError {
                    message: "exponent has no digits".to_string(),
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }));
            }
        }

        let digits_end = self.pos;
        while let Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') = self.peek() {
            self.advance();
        }
        let suffix_str = &self.source[digits_end..self.pos];
        let suffix = match Self::parse_float_suffix(suffix_str) {
            Ok(s) => s,
            Err(_) => {
                return Some(Err(LexError {
                    message: "invalid suffix on floating constant".to_string(),
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }));
            }
        };

        let raw = &self.source[start..digits_end];
        let filtered: String = raw.chars().filter(|&c| c != '\'').collect();
        match filtered.parse::<f64>() {
            Ok(value) => Some(Ok(Token {
                kind: TokenKind::FloatConstant { value, suffix },
                span: Span {
                    start,
                    end: self.pos,
                },
            })),
            Err(_) => Some(Err(LexError {
                message: "floating constant cannot be parsed".to_string(),
                span: Span {
                    start,
                    end: self.pos,
                },
            })),
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
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'%' => TokenKind::Percent,
            b'~' => TokenKind::Tilde,
            b'!' => TokenKind::Bang,
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
                let has_sep =
                    match self.consume_digits_with_separators(|b| b.is_ascii_digit(), true) {
                        Ok(s) => s,
                        Err(msg) => {
                            return Some(Err(LexError {
                                message: msg.to_string(),
                                span: Span {
                                    start,
                                    end: self.pos,
                                },
                            }));
                        }
                    };
                if self.peek() == Some(b'.') || matches!(self.peek(), Some(b'e' | b'E')) {
                    return self.finish_float_constant(start);
                }
                return self.finish_int_constant(start, start, 10, IntBase::Decimal, has_sep);
            }
            b'0' => match self.peek() {
                Some(b'x' | b'X') => {
                    self.advance();
                    let digits_start = self.pos;
                    let has_sep = match self
                        .consume_digits_with_separators(|b| b.is_ascii_hexdigit(), false)
                    {
                        Ok(s) => s,
                        Err(msg) => {
                            return Some(Err(LexError {
                                message: msg.to_string(),
                                span: Span {
                                    start,
                                    end: self.pos,
                                },
                            }));
                        }
                    };
                    if self.pos == digits_start {
                        return Some(Err(LexError {
                            message: "hexadecimal constant has no digits".to_string(),
                            span: Span {
                                start,
                                end: self.pos,
                            },
                        }));
                    }
                    return self.finish_int_constant(
                        start,
                        digits_start,
                        16,
                        IntBase::Hex,
                        has_sep,
                    );
                }
                Some(b'b' | b'B') => {
                    self.advance();
                    let digits_start = self.pos;
                    let has_sep = match self
                        .consume_digits_with_separators(|b| b == b'0' || b == b'1', false)
                    {
                        Ok(s) => s,
                        Err(msg) => {
                            return Some(Err(LexError {
                                message: msg.to_string(),
                                span: Span {
                                    start,
                                    end: self.pos,
                                },
                            }));
                        }
                    };
                    if self.pos == digits_start {
                        return Some(Err(LexError {
                            message: "binary constant has no digits".to_string(),
                            span: Span {
                                start,
                                end: self.pos,
                            },
                        }));
                    }
                    return self.finish_int_constant(
                        start,
                        digits_start,
                        2,
                        IntBase::Binary,
                        has_sep,
                    );
                }
                _ => {
                    if self.peek() == Some(b'.') || matches!(self.peek(), Some(b'e' | b'E')) {
                        return self.finish_float_constant(start);
                    }
                    let digits_start = start;
                    let has_sep =
                        match self.consume_digits_with_separators(|b| b.is_ascii_digit(), true) {
                            Ok(s) => s,
                            Err(msg) => {
                                return Some(Err(LexError {
                                    message: msg.to_string(),
                                    span: Span {
                                        start,
                                        end: self.pos,
                                    },
                                }));
                            }
                        };
                    let base = if self.pos == start + 1 {
                        IntBase::Decimal
                    } else {
                        if self.source[digits_start..self.pos]
                            .bytes()
                            .any(|b| b == b'8' || b == b'9')
                        {
                            return Some(Err(LexError {
                                message: "invalid digit in octal constant".to_string(),
                                span: Span {
                                    start,
                                    end: self.pos,
                                },
                            }));
                        }
                        IntBase::Octal
                    };
                    let radix = if base == IntBase::Decimal { 10 } else { 8 };
                    return self.finish_int_constant(start, digits_start, radix, base, has_sep);
                }
            },
            b'.' => {
                if let Some(b'0'..=b'9') = self.peek() {
                    return self.finish_float_constant(start);
                }
                return Some(Err(LexError {
                    message: "unexpected character '.'".to_string(),
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }));
            }
            _ => {
                //decode the full UTF-8 char starting from `start`. works even if the invalid
                //character is ASCII.
                let ch = self.source[start..].chars().next().unwrap();
                self.pos = start + ch.len_utf8();
                return Some(Err(LexError {
                    message: format!("unexpected character '{ch}'"),
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }));
            }
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

    #[test]
    fn zero_and_bases_are_parsed() {
        let cases = [
            ("0;", 0, IntBase::Decimal, IntSuffix::Unsuffixed, 1),
            ("0u;", 0, IntBase::Decimal, IntSuffix::U, 2),
            ("0x1f;", 0x1f, IntBase::Hex, IntSuffix::Unsuffixed, 4),
            ("0X2Au;", 0x2a, IntBase::Hex, IntSuffix::U, 5),
            ("077;", 0o77, IntBase::Octal, IntSuffix::Unsuffixed, 3),
            ("0b1011;", 0b1011, IntBase::Binary, IntSuffix::Unsuffixed, 6),
        ];

        for (src, val, base, suffix, len) in cases {
            let token = Lexer::new(src).next().unwrap().unwrap();
            assert_eq!(
                token.kind,
                TokenKind::Constant {
                    value: val,
                    suffix,
                    base,
                }
            );
            assert_eq!(token.span, Span { start: 0, end: len });
        }
    }

    #[test]
    fn invalid_bases_produce_errors() {
        let cases = [
            ("0x;", "hexadecimal constant has no digits", 2),
            ("0b;", "binary constant has no digits", 2),
            ("08;", "invalid digit in octal constant", 2),
            ("0x1g;", "invalid suffix on integer constant", 4),
        ];

        for (src, msg, len) in cases {
            let err = Lexer::new(src).next().unwrap().unwrap_err();
            assert_eq!(err.message, msg);
            assert_eq!(err.span, Span { start: 0, end: len });
        }
    }

    #[test]
    fn unexpected_ascii_character_is_an_error() {
        let err = Lexer::new("@").next().unwrap().unwrap_err();
        assert_eq!(err.message, "unexpected character '@'");
        assert_eq!(err.span, Span { start: 0, end: 1 });
    }

    #[test]
    fn unexpected_multibyte_character_has_correct_span_and_recovers() {
        //'🦀' is 4 bytes in UTF-8
        let mut lexer = Lexer::new("🦀 42;");

        let err = lexer.next().unwrap().unwrap_err();
        assert_eq!(err.message, "unexpected character '🦀'");
        assert_eq!(err.span, Span { start: 0, end: 4 });

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(
            token.kind,
            TokenKind::Constant {
                value: 42,
                suffix: IntSuffix::Unsuffixed,
                base: IntBase::Decimal,
            }
        );
        assert_eq!(token.span, Span { start: 5, end: 7 });
    }

    #[test]
    fn integer_constants_with_digit_separators() {
        let cases = [
            ("1'000'000;", 1_000_000, IntBase::Decimal),
            ("0xde'ad'be'ef;", 0xdeadbeef, IntBase::Hex),
            ("0b10'11;", 0b1011, IntBase::Binary),
            ("0'77;", 0o77, IntBase::Octal),
        ];

        for (src, val, base) in cases {
            let token = Lexer::new(src).next().unwrap().unwrap();
            assert_eq!(
                token.kind,
                TokenKind::Constant {
                    value: val,
                    suffix: IntSuffix::Unsuffixed,
                    base,
                }
            );
        }
    }

    #[test]
    fn invalid_digit_separator_placement_is_rejected() {
        let cases = [
            ("1''0;", "consecutive digit separators are invalid"),
            ("100';", "digit separator cannot appear at end of number"),
        ];

        for (src, msg) in cases {
            let err = Lexer::new(src).next().unwrap().unwrap_err();
            assert_eq!(err.message, msg);
        }
    }

    #[test]
    fn decimal_float_constants_are_parsed() {
        let cases = [
            ("3.25;", 3.25, FloatSuffix::Unsuffixed, 4),
            ("1.0f;", 1.0, FloatSuffix::F, 4),
            (".5L;", 0.5, FloatSuffix::L, 3),
            ("1e3;", 1000.0, FloatSuffix::Unsuffixed, 3),
            ("2.5e-2f;", 0.025, FloatSuffix::F, 7),
            ("42.;", 42.0, FloatSuffix::Unsuffixed, 3),
        ];

        for (src, val, suffix, len) in cases {
            let token = Lexer::new(src).next().unwrap().unwrap();
            match token.kind {
                TokenKind::FloatConstant {
                    value,
                    suffix: actual_suffix,
                } => {
                    assert_eq!(actual_suffix, suffix);
                    assert!((value - val).abs() < 1e-9);
                }
                other => panic!("expected FloatConstant, got {other:?} for {src}"),
            }
            assert_eq!(token.span, Span { start: 0, end: len });
        }
    }

    #[test]
    fn arithmetic_operators_are_lexed() {
        let src = "+ - * / % ~ !";
        let kinds: Vec<TokenKind> = Lexer::new(src).map(|t| t.unwrap().kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Tilde,
                TokenKind::Bang,
                TokenKind::Eof,
            ]
        );
    }
}
