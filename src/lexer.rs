use crate::token::Token;

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            input: source.chars().collect(),
            pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token == Token::Eof;

            tokens.push(token);

            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace_and_comments();

        let ch = match self.current() {
            Some(ch) => ch,
            None => return Ok(Token::Eof),
        };

        match ch {
            '=' => {
                self.advance();

                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::EqEq)
                } else {
                    Ok(Token::Eq)
                }
            }

            '!' => {
                self.advance();

                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::BangEq)
                } else {
                    Ok(Token::Bang)
                }
            }

            '<' => {
                self.advance();

                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::LessEq)
                } else {
                    Ok(Token::Less)
                }
            }

            '>' => {
                self.advance();

                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::GreaterEq)
                } else {
                    Ok(Token::Greater)
                }
            }

            ';' => {
                self.advance();
                Ok(Token::Semi)
            }

            ',' => {
                self.advance();
                Ok(Token::Comma)
            }

            '+' => {
                self.advance();
                Ok(Token::Plus)
            }

            '-' => {
                self.advance();
                Ok(Token::Minus)
            }

            '*' => {
                self.advance();
                Ok(Token::Star)
            }

            '/' => {
                self.advance();
                Ok(Token::Slash)
            }

            '(' => {
                self.advance();
                Ok(Token::LParen)
            }

            ')' => {
                self.advance();
                Ok(Token::RParen)
            }

            '{' => {
                self.advance();
                Ok(Token::LBrace)
            }

            '}' => {
                self.advance();
                Ok(Token::RBrace)
            }

            '"' => self.read_string(),

            ch if ch.is_ascii_digit() => self.read_number(),

            ch if is_ident_start(ch) => Ok(self.read_identifier_or_keyword()),

            _ => Err(format!("Unexpected character: '{}'", ch)),
        }
    }

    fn read_identifier_or_keyword(&mut self) -> Token {
        let start = self.pos;

        while let Some(ch) = self.current() {
            if is_ident_part(ch) {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.pos].iter().collect();

        match text.as_str() {
            "pub" => Token::Pub,
            "fcn" => Token::Fcn,
            "if" => Token::If,
            "else" => Token::Else,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),

            _ if text.starts_with("cre_") => {
                let ty = text.trim_start_matches("cre_").to_string();
                Token::CreateType(ty)
            }

            _ => Token::Ident(text),
        }
    }

    fn read_number(&mut self) -> Result<Token, String> {
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
            Ok(value) => Ok(Token::Int(value)),
            Err(_) => Err(format!("Invalid integer literal: {}", text)),
        }
    }

    fn read_string(&mut self) -> Result<Token, String> {
        self.advance();

        let mut value = String::new();

        while let Some(ch) = self.current() {
            match ch {
                '"' => {
                    self.advance();
                    return Ok(Token::Str(value));
                }

                '\\' => {
                    self.advance();

                    let escaped = match self.current() {
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('"') => '"',
                        Some('\\') => '\\',

                        Some(other) => {
                            return Err(format!("Invalid escape sequence: \\{}", other));
                        }

                        None => {
                            return Err("Unterminated escape sequence in string".to_string());
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

        Err("Unterminated string literal".to_string())
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
        self.pos += 1;
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}