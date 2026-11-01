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
}
