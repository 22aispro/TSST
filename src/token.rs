#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Eof,

    Ident(String),
    Int(i64),
    Str(String),
    Bool(bool),
    Type(String),

    Pub,
    Fcn,
    Return,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    Use,

    Plus,
    Minus,
    Star,
    Slash,

    Equal,
    EqualEqual,
    Bang,
    BangEqual,

    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    AndAnd,
    OrOr,

    Arrow,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Comma,
    Colon,
    Semicolon,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    #[allow(dead_code)]
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: String, line: usize, column: usize) -> Self {
        Self {
            kind,
            lexeme,
            line,
            column,
        }
    }
}
