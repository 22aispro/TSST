use crate::ast::*;
use crate::token::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();

        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }

        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        match self.current() {
            Token::CreateType(_) => {
                let var_decl = self.parse_var_decl()?;
                Ok(Item::VarDecl(var_decl))
            }

            Token::Pub | Token::Fcn => {
                let function = self.parse_function()?;
                Ok(Item::Function(function))
            }

            other => Err(format!("Expected item, found {:?}", other)),
        }
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.current() {
            Token::CreateType(_) => {
                let var_decl = self.parse_var_decl()?;
                Ok(Stmt::VarDecl(var_decl))
            }

            Token::If => self.parse_if(),

            Token::Ident(_) => {
                if self.peek_token() == Token::Eq {
                    self.parse_assignment()
                } else {
                    self.parse_macro_call()
                }
            }

            other => Err(format!("Expected statement, found {:?}", other)),
        }
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, String> {
        let ty = match self.advance() {
            Token::CreateType(ty) => ty,
            other => return Err(format!("Expected variable type, found {:?}", other)),
        };

        let name = match self.advance() {
            Token::Ident(name) => name,
            other => return Err(format!("Expected variable name, found {:?}", other)),
        };

        self.expect(Token::Eq)?;

        let value = self.parse_expr()?;

        self.expect(Token::Semi)?;

        Ok(VarDecl { ty, name, value })
    }

    fn parse_assignment(&mut self) -> Result<Stmt, String> {
        let name = match self.advance() {
            Token::Ident(name) => name,
            other => return Err(format!("Expected variable name, found {:?}", other)),
        };

        self.expect(Token::Eq)?;

        let value = self.parse_expr()?;

        self.expect(Token::Semi)?;

        Ok(Stmt::Assignment(Assignment { name, value }))
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        let public = self.matches(&Token::Pub);

        self.expect(Token::Fcn)?;

        let name = match self.advance() {
            Token::Ident(name) => name,
            other => return Err(format!("Expected function name, found {:?}", other)),
        };

        self.expect(Token::LParen)?;
        self.expect(Token::RParen)?;

        let body = self.parse_block()?;

        Ok(Function {
            public,
            name,
            body,
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(Token::If)?;

        let condition = self.parse_expr()?;

        let then_body = self.parse_block()?;

        let else_body = if self.matches(&Token::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::If(IfStmt {
            condition,
            then_body,
            else_body,
        }))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::LBrace)?;

        let mut body = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
        }

        self.expect(Token::RBrace)?;

        Ok(body)
    }

    fn parse_macro_call(&mut self) -> Result<Stmt, String> {
        let name = match self.advance() {
            Token::Ident(name) => name,
            other => return Err(format!("Expected macro name, found {:?}", other)),
        };

        self.expect(Token::Bang)?;
        self.expect(Token::LParen)?;

        let mut args = Vec::new();

        if !self.check(&Token::RParen) {
            args.push(self.parse_expr()?);

            while self.matches(&Token::Comma) {
                args.push(self.parse_expr()?);
            }
        }

        self.expect(Token::RParen)?;
        self.expect(Token::Semi)?;

        Ok(Stmt::MacroCall(MacroCall { name, args }))
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_equality()
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_comparison()?;

        loop {
            let op = match self.current() {
                Token::EqEq => BinaryOp::Eq,
                Token::BangEq => BinaryOp::NotEq,
                _ => break,
            };

            self.advance();

            let right = self.parse_comparison()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_addition()?;

        loop {
            let op = match self.current() {
                Token::Less => BinaryOp::Less,
                Token::Greater => BinaryOp::Greater,
                Token::LessEq => BinaryOp::LessEq,
                Token::GreaterEq => BinaryOp::GreaterEq,
                _ => break,
            };

            self.advance();

            let right = self.parse_addition()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_multiplication()?;

        loop {
            let op = match self.current() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance();

            let right = self.parse_multiplication()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            let op = match self.current() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };

            self.advance();

            let right = self.parse_primary()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Token::Int(value) => Ok(Expr::Int(value)),
            Token::Str(value) => Ok(Expr::Str(value)),
            Token::Bool(value) => Ok(Expr::Bool(value)),
            Token::Ident(name) => Ok(Expr::Ident(name)),

            Token::LParen => {
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }

            other => Err(format!("Expected expression, found {:?}", other)),
        }
    }

    fn current(&self) -> Token {
        self.tokens
            .get(self.pos)
            .cloned()
            .unwrap_or(Token::Eof)
    }

    fn peek_token(&self) -> Token {
        self.tokens
            .get(self.pos + 1)
            .cloned()
            .unwrap_or(Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.current();

        if !self.is_at_end() {
            self.pos += 1;
        }

        token
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        let actual = self.advance();

        if actual == expected {
            Ok(())
        } else {
            Err(format!("Expected {:?}, found {:?}", expected, actual))
        }
    }

    fn check(&self, expected: &Token) -> bool {
        &self.current() == expected
    }

    fn matches(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn is_at_end(&self) -> bool {
        self.current() == Token::Eof
    }
}