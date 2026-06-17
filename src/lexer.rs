use crate::token::{Token, TokenKind};

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            input: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;

            tokens.push(token);

            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let column = self.column;

        let ch = match self.current() {
            Some(ch) => ch,
            None => return Ok(Token::new(TokenKind::Eof, line, column)),
        };

        match ch {
            '=' => {
                self.advance();

                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::EqEq, line, column))
                } else {
                    Ok(Token::new(TokenKind::Eq, line, column))
                }
            }

            '!' => {
                self.advance();

                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::BangEq, line, column))
                } else {
                    Ok(Token::new(TokenKind::Bang, line, column))
                }
            }

            '<' => {
                self.advance();

                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::LessEq, line, column))
                } else {
                    Ok(Token::new(TokenKind::Less, line, column))
                }
            }

            '>' => {
                self.advance();

                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::GreaterEq, line, column))
                } else {
                    Ok(Token::new(TokenKind::Greater, line, column))
                }
            }

            '-' => {
                self.advance();

                if self.current() == Some('>') {
                    self.advance();
                    Ok(Token::new(TokenKind::Arrow, line, column))
                } else {
                    Ok(Token::new(TokenKind::Minus, line, column))
                }
            }

            ';' => {
                self.advance();
                Ok(Token::new(TokenKind::Semi, line, column))
            }

            ',' => {
                self.advance();
                Ok(Token::new(TokenKind::Comma, line, column))
            }

            ':' => {
                self.advance();
                Ok(Token::new(TokenKind::Colon, line, column))
            }

            '+' => {
                self.advance();
                Ok(Token::new(TokenKind::Plus, line, column))
            }

            '*' => {
                self.advance();
                Ok(Token::new(TokenKind::Star, line, column))
            }

            '/' => {
                self.advance();
                Ok(Token::new(TokenKind::Slash, line, column))
            }

            '(' => {
                self.advance();
                Ok(Token::new(TokenKind::LParen, line, column))
            }

            ')' => {
                self.advance();
                Ok(Token::new(TokenKind::RParen, line, column))
            }

            '{' => {
                self.advance();
                Ok(Token::new(TokenKind::LBrace, line, column))
            }

            '}' => {
                self.advance();
                Ok(Token::new(TokenKind::RBrace, line, column))
            }

            '[' => {
                self.advance();
                Ok(Token::new(TokenKind::LBracket, line, column))
            }

            ']' => {
                self.advance();
                Ok(Token::new(TokenKind::RBracket, line, column))
            }

            '"' => self.read_string(line, column),

            ch if ch.is_ascii_digit() => self.read_number(line, column),

            ch if is_ident_start(ch) => Ok(self.read_identifier_or_keyword(line, column)),

            _ => Err(format!(
                "line {}, column {}: Unexpected character '{}'",
                line, column, ch
            )),
        }
    }

    fn read_identifier_or_keyword(&mut self, line: usize, column: usize) -> Token {
        let start = self.pos;

        while let Some(ch) = self.current() {
            if is_ident_part(ch) {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.pos].iter().collect();

        let kind = match text.as_str() {
            "pub" => TokenKind::Pub,
            "fcn" => TokenKind::Fcn,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),

            _ if text.starts_with("cre_") => {
                let ty = text.trim_start_matches("cre_").to_string();
                TokenKind::CreateType(ty)
            }

            _ => TokenKind::Ident(text),
        };

        Token::new(kind, line, column)
    }

    fn read_number(&mut self, line: usize, column: usize) -> Result<Token, String> {
        let start = self.pos;

        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.pos].iter().collect();

        match text.parse::<i64>() {
            Ok(value) => Ok(Token::new(TokenKind::Int(value), line, column)),
            Err(_) => Err(format!(
                "line {}, column {}: Invalid integer literal '{}'",
                line, column, text
            )),
        }
    }

    fn read_string(&mut self, line: usize, column: usize) -> Result<Token, String> {
        self.advance();

        let mut value = String::new();

        while let Some(ch) = self.current() {
            match ch {
                '"' => {
                    self.advance();
                    return Ok(Token::new(TokenKind::Str(value), line, column));
                }

                '\\' => {
                    self.advance();

                    let escaped = match self.current() {
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('"') => '"',
                        Some('\\') => '\\',

                        Some(other) => {
                            return Err(format!(
                                "line {}, column {}: Invalid escape sequence '\\{}'",
                                self.line, self.column, other
                            ));
                        }

                        None => {
                            return Err(format!(
                                "line {}, column {}: Unterminated escape sequence in string",
                                line, column
                            ));
                        }
                    };

                    value.push(escaped);
                    self.advance();
                }

                other => {
                    value.push(other);
                    self.advance();
                }
            }
        }

        Err(format!(
            "line {}, column {}: Unterminated string literal",
            line, column
        ))
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while let Some(ch) = self.current() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            if self.current() == Some('/') && self.peek() == Some('/') {
                while let Some(ch) = self.current() {
                    self.advance();

                    if ch == '\n' {
                        break;
                    }
                }

                continue;
            }

            break;
        }
    }

    fn current(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current() {
            self.pos += 1;

            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}