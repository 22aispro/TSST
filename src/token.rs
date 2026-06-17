#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Pub,
    Fcn,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    Return,

    CreateType(String),

    Ident(String),
    Int(i64),
    Str(String),
    Bool(bool),

    Eq,
    EqEq,
    Bang,
    BangEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    Arrow,

    Semi,
    Comma,
    Colon,

    Plus,
    Minus,
    Star,
    Slash,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }
}