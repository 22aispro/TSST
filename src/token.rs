#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Pub,
    Fcn,
    If,
    Else,

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

    Semi,
    Comma,

    Plus,
    Minus,
    Star,
    Slash,

    LParen,
    RParen,
    LBrace,
    RBrace,

    Eof,
}