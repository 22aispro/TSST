use crate::token::{Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            let char_value = self.peek();
            let line = self.line;
            let column = self.column;

            match char_value {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }

                '\n' => {
                    self.advance_newline();
                }

                '/' => {
                    if self.peek_next() == '/' {
                        self.skip_comment();
                    } else {
                        self.advance();
                        tokens.push(Token::new(TokenKind::Slash, "/".to_string(), line, column));
                    }
                }

                '+' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Plus, "+".to_string(), line, column));
                }

                '-' => {
                    self.advance();

                    if self.match_char('>') {
                        tokens.push(Token::new(TokenKind::Arrow, "->".to_string(), line, column));
                    } else {
                        tokens.push(Token::new(TokenKind::Minus, "-".to_string(), line, column));
                    }
                }

                '*' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Star, "*".to_string(), line, column));
                }

                '=' => {
                    self.advance();

                    if self.match_char('=') {
                        tokens.push(Token::new(
                            TokenKind::EqualEqual,
                            "==".to_string(),
                            line,
                            column,
                        ));
                    } else {
                        tokens.push(Token::new(TokenKind::Equal, "=".to_string(), line, column));
                    }
                }

                '!' => {
                    self.advance();

                    if self.match_char('=') {
                        tokens.push(Token::new(
                            TokenKind::BangEqual,
                            "!=".to_string(),
                            line,
                            column,
                        ));
                    } else {
                        tokens.push(Token::new(TokenKind::Bang, "!".to_string(), line, column));
                    }
                }

                '<' => {
                    self.advance();

                    if self.match_char('=') {
                        tokens.push(Token::new(
                            TokenKind::LessEqual,
                            "<=".to_string(),
                            line,
                            column,
                        ));
                    } else {
                        tokens.push(Token::new(TokenKind::Less, "<".to_string(), line, column));
                    }
                }

                '>' => {
                    self.advance();

                    if self.match_char('=') {
                        tokens.push(Token::new(
                            TokenKind::GreaterEqual,
                            ">=".to_string(),
                            line,
                            column,
                        ));
                    } else {
                        tokens.push(Token::new(
                            TokenKind::Greater,
                            ">".to_string(),
                            line,
                            column,
                        ));
                    }
                }

                '&' => {
                    self.advance();

                    if self.match_char('&') {
                        tokens.push(Token::new(
                            TokenKind::AndAnd,
                            "&&".to_string(),
                            line,
                            column,
                        ));
                    } else {
                        return Err(format!(
                            "line {line}, column {column}: Expected '&' after '&'. Use &&."
                        ));
                    }
                }

                '|' => {
                    self.advance();

                    if self.match_char('|') {
                        tokens.push(Token::new(TokenKind::OrOr, "||".to_string(), line, column));
                    } else {
                        return Err(format!(
                            "line {line}, column {column}: Expected '|' after '|'. Use ||."
                        ));
                    }
                }

                '(' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::LParen, "(".to_string(), line, column));
                }

                ')' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::RParen, ")".to_string(), line, column));
                }

                '{' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::LBrace, "{".to_string(), line, column));
                }

                '}' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::RBrace, "}".to_string(), line, column));
                }

                '[' => {
                    self.advance();
                    tokens.push(Token::new(
                        TokenKind::LBracket,
                        "[".to_string(),
                        line,
                        column,
                    ));
                }

                ']' => {
                    self.advance();
                    tokens.push(Token::new(
                        TokenKind::RBracket,
                        "]".to_string(),
                        line,
                        column,
                    ));
                }

                ',' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Comma, ",".to_string(), line, column));
                }

                ':' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Colon, ":".to_string(), line, column));
                }

                ';' => {
                    self.advance();
                    tokens.push(Token::new(
                        TokenKind::Semicolon,
                        ";".to_string(),
                        line,
                        column,
                    ));
                }

                '"' => {
                    tokens.push(self.string()?);
                }

                value if value.is_ascii_digit() => {
                    tokens.push(self.number()?);
                }

                value if is_identifier_start(value) => {
                    tokens.push(self.identifier());
                }

                other => {
                    return Err(format!(
                        "line {line}, column {column}: Unexpected character '{other}'."
                    ));
                }
            }
        }

        tokens.push(Token::new(
            TokenKind::Eof,
            String::new(),
            self.line,
            self.column,
        ));

        Ok(tokens)
    }

    fn string(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_column = self.column;

        self.advance();

        let mut value = String::new();

        while !self.is_at_end() {
            let char_value = self.peek();

            if char_value == '"' {
                self.advance();

                return Ok(Token::new(
                    TokenKind::Str(value.clone()),
                    value,
                    start_line,
                    start_column,
                ));
            }

            if char_value == '\\' {
                self.advance();

                if self.is_at_end() {
                    return Err(format!(
                        "line {start_line}, column {start_column}: Unterminated escape sequence."
                    ));
                }

                let escaped = self.peek();

                match escaped {
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    other => value.push(other),
                }

                self.advance();
                continue;
            }

            if char_value == '\n' {
                value.push('\n');
                self.advance_newline();
            } else {
                value.push(char_value);
                self.advance();
            }
        }

        Err(format!(
            "line {start_line}, column {start_column}: Unterminated string."
        ))
    }

    fn number(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();

        while !self.is_at_end() && self.peek().is_ascii_digit() {
            value.push(self.peek());
            self.advance();
        }

        let parsed = value.parse::<i64>().map_err(|error| {
            format!("line {start_line}, column {start_column}: Invalid integer '{value}': {error}")
        })?;

        Ok(Token::new(
            TokenKind::Int(parsed),
            value,
            start_line,
            start_column,
        ))
    }

    fn identifier(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();

        while !self.is_at_end() && is_identifier_part(self.peek()) {
            value.push(self.peek());
            self.advance();
        }

        let kind = match value.as_str() {
            "pub" => TokenKind::Pub,
            "fcn" => TokenKind::Fcn,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "use" => TokenKind::Use,

            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),

            "cre_int" | "cre_str" | "cre_bool" | "cre_arr" | "cre_dict" => {
                TokenKind::Type(value.clone())
            }

            _ => TokenKind::Ident(value.clone()),
        };

        Token::new(kind, value, start_line, start_column)
    }

    fn skip_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
    }

    fn advance(&mut self) -> char {
        let value = self.chars[self.current];
        self.current += 1;
        self.column += 1;
        value
    }

    fn advance_newline(&mut self) {
        self.current += 1;
        self.line += 1;
        self.column = 1;
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        if self.peek() != expected {
            return false;
        }

        self.advance();
        true
    }

    fn peek(&self) -> char {
        self.chars[self.current]
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.chars.len() {
            '\0'
        } else {
            self.chars[self.current + 1]
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.chars.len()
    }
}

fn is_identifier_start(value: char) -> bool {
    value.is_ascii_alphabetic() || value == '_'
}

fn is_identifier_part(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}
