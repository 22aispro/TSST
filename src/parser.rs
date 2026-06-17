use crate::ast::*;
use crate::token::{Token, TokenKind};

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
            TokenKind::CreateType(_) => {
                let var_decl = self.parse_var_decl()?;
                Ok(Item::VarDecl(var_decl))
            }

            TokenKind::Pub | TokenKind::Fcn => {
                let function = self.parse_function()?;
                Ok(Item::Function(function))
            }

            other => self.error(format!("Expected item, found {:?}", other)),
        }
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.current() {
            TokenKind::CreateType(_) => {
                let var_decl = self.parse_var_decl()?;
                Ok(Stmt::VarDecl(var_decl))
            }

            TokenKind::If => self.parse_if(),

            TokenKind::While => self.parse_while(),

            TokenKind::For => self.parse_for(),

            TokenKind::Break => {
                self.advance();
                self.expect(TokenKind::Semi)?;
                Ok(Stmt::Break)
            }

            TokenKind::Continue => {
                self.advance();
                self.expect(TokenKind::Semi)?;
                Ok(Stmt::Continue)
            }

            TokenKind::Return => self.parse_return(),

            TokenKind::Ident(_) => {
                if self.peek_token() == TokenKind::Eq {
                    self.parse_assignment()
                } else if self.peek_token() == TokenKind::Bang {
                    self.parse_macro_call()
                } else if self.peek_token() == TokenKind::LParen {
                    self.parse_function_call_stmt()
                } else {
                    self.error("Expected assignment, macro call, or function call".to_string())
                }
            }

            other => self.error(format!("Expected statement, found {:?}", other)),
        }
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, String> {
        let var_decl = self.parse_var_decl_no_semi()?;
        self.expect(TokenKind::Semi)?;
        Ok(var_decl)
    }

    fn parse_var_decl_no_semi(&mut self) -> Result<VarDecl, String> {
        let ty = match self.advance().kind {
            TokenKind::CreateType(ty) => ty,
            other => return self.error(format!("Expected variable type, found {:?}", other)),
        };

        let name = match self.advance().kind {
            TokenKind::Ident(name) => name,
            other => return self.error(format!("Expected variable name, found {:?}", other)),
        };

        self.expect(TokenKind::Eq)?;

        let value = self.parse_expr()?;

        Ok(VarDecl { ty, name, value })
    }

    fn parse_assignment(&mut self) -> Result<Stmt, String> {
        let assignment = self.parse_assignment_no_semi()?;
        self.expect(TokenKind::Semi)?;
        Ok(Stmt::Assignment(assignment))
    }

    fn parse_assignment_no_semi(&mut self) -> Result<Assignment, String> {
        let name = match self.advance().kind {
            TokenKind::Ident(name) => name,
            other => return self.error(format!("Expected variable name, found {:?}", other)),
        };

        self.expect(TokenKind::Eq)?;

        let value = self.parse_expr()?;

        Ok(Assignment { name, value })
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        let public = self.matches(&TokenKind::Pub);

        self.expect(TokenKind::Fcn)?;

        let name = match self.advance().kind {
            TokenKind::Ident(name) => name,
            other => return self.error(format!("Expected function name, found {:?}", other)),
        };

        self.expect(TokenKind::LParen)?;

        let mut params = Vec::new();

        if !self.check(&TokenKind::RParen) {
            loop {
                params.push(self.parse_param()?);

                if self.matches(&TokenKind::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(TokenKind::RParen)?;

        let return_type = if self.matches(&TokenKind::Arrow) {
            Some(self.parse_type_name()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Function {
            public,
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_param(&mut self) -> Result<Param, String> {
        let ty = match self.advance().kind {
            TokenKind::CreateType(ty) => ty,
            other => return self.error(format!("Expected parameter type, found {:?}", other)),
        };

        let name = match self.advance().kind {
            TokenKind::Ident(name) => name,
            other => return self.error(format!("Expected parameter name, found {:?}", other)),
        };

        Ok(Param { ty, name })
    }

    fn parse_type_name(&mut self) -> Result<String, String> {
        match self.advance().kind {
            TokenKind::Ident(name) => Ok(name),
            TokenKind::CreateType(ty) => Ok(ty),
            other => self.error(format!("Expected type name, found {:?}", other)),
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::If)?;

        let condition = self.parse_expr()?;

        let then_body = self.parse_block()?;

        let else_body = if self.matches(&TokenKind::Else) {
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

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::While)?;

        let condition = self.parse_expr()?;

        let body = self.parse_block()?;

        Ok(Stmt::While(WhileStmt { condition, body }))
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::For)?;
        self.expect(TokenKind::LParen)?;

        let ty = match self.advance().kind {
            TokenKind::CreateType(ty) => ty,
            other => return self.error(format!("Expected for-loop variable type, found {:?}", other)),
        };

        let name = match self.advance().kind {
            TokenKind::Ident(name) => name,
            other => return self.error(format!("Expected for-loop variable name, found {:?}", other)),
        };

        match self.current() {
            TokenKind::In => {
                self.advance();

                let iterable = self.parse_expr()?;

                self.expect(TokenKind::RParen)?;

                let body = self.parse_block()?;

                Ok(Stmt::ForEach(ForEachStmt {
                    item_ty: ty,
                    item_name: name,
                    iterable,
                    body,
                }))
            }

            TokenKind::Eq => {
                self.advance();

                let value = self.parse_expr()?;

                let initializer = VarDecl {
                    ty,
                    name,
                    value,
                };

                self.expect(TokenKind::Semi)?;

                let condition = self.parse_expr()?;

                self.expect(TokenKind::Semi)?;

                let update = self.parse_assignment_no_semi()?;

                self.expect(TokenKind::RParen)?;

                let body = self.parse_block()?;

                Ok(Stmt::For(ForStmt {
                    initializer,
                    condition,
                    update,
                    body,
                }))
            }

            other => self.error(format!("Expected 'in' or '=' in for loop, found {:?}", other)),
        }
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::Return)?;

        let value = self.parse_expr()?;

        self.expect(TokenKind::Semi)?;

        Ok(Stmt::Return(ReturnStmt { value }))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
        }

        self.expect(TokenKind::RBrace)?;

        Ok(body)
    }

    fn parse_macro_call(&mut self) -> Result<Stmt, String> {
        let name = match self.advance().kind {
            TokenKind::Ident(name) => name,
            other => return self.error(format!("Expected macro name, found {:?}", other)),
        };

        self.expect(TokenKind::Bang)?;
        self.expect(TokenKind::LParen)?;

        let args = self.parse_call_args_after_lparen()?;

        self.expect(TokenKind::Semi)?;

        Ok(Stmt::MacroCall(MacroCall { name, args }))
    }

    fn parse_function_call_stmt(&mut self) -> Result<Stmt, String> {
        let name = match self.advance().kind {
            TokenKind::Ident(name) => name,
            other => return self.error(format!("Expected function name, found {:?}", other)),
        };

        self.expect(TokenKind::LParen)?;

        let args = self.parse_call_args_after_lparen()?;

        self.expect(TokenKind::Semi)?;

        Ok(Stmt::FunctionCall(FunctionCall { name, args }))
    }

    fn parse_call_args_after_lparen(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();

        if !self.check(&TokenKind::RParen) {
            args.push(self.parse_expr()?);

            while self.matches(&TokenKind::Comma) {
                args.push(self.parse_expr()?);
            }
        }

        self.expect(TokenKind::RParen)?;

        Ok(args)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_equality()
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_comparison()?;

        loop {
            let op = match self.current() {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::BangEq => BinaryOp::NotEq,
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
                TokenKind::Less => BinaryOp::Less,
                TokenKind::Greater => BinaryOp::Greater,
                TokenKind::LessEq => BinaryOp::LessEq,
                TokenKind::GreaterEq => BinaryOp::GreaterEq,
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
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
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
        let mut expr = self.parse_postfix()?;

        loop {
            let op = match self.current() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => break,
            };

            self.advance();

            let right = self.parse_postfix()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.matches(&TokenKind::LBracket) {
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;

                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(index),
                };

                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance().kind {
            TokenKind::Int(value) => Ok(Expr::Int(value)),
            TokenKind::Str(value) => Ok(Expr::Str(value)),
            TokenKind::Bool(value) => Ok(Expr::Bool(value)),

            TokenKind::Ident(name) => {
                if self.check(&TokenKind::LParen) {
                    self.expect(TokenKind::LParen)?;
                    let args = self.parse_call_args_after_lparen()?;

                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }

            TokenKind::LBracket => {
                let mut items = Vec::new();

                if !self.check(&TokenKind::RBracket) {
                    items.push(self.parse_expr()?);

                    while self.matches(&TokenKind::Comma) {
                        items.push(self.parse_expr()?);
                    }
                }

                self.expect(TokenKind::RBracket)?;

                Ok(Expr::ArrayLiteral(items))
            }

            TokenKind::LBrace => {
                let mut pairs = Vec::new();

                if !self.check(&TokenKind::RBrace) {
                    loop {
                        let key = match self.advance().kind {
                            TokenKind::Str(key) => key,
                            other => {
                                return self.error(format!(
                                    "Expected string key in dictionary, found {:?}",
                                    other
                                ));
                            }
                        };

                        self.expect(TokenKind::Colon)?;

                        let value = self.parse_expr()?;

                        pairs.push((key, value));

                        if self.matches(&TokenKind::Comma) {
                            if self.check(&TokenKind::RBrace) {
                                break;
                            }

                            continue;
                        }

                        break;
                    }
                }

                self.expect(TokenKind::RBrace)?;

                Ok(Expr::DictLiteral(pairs))
            }

            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }

            other => self.error(format!("Expected expression, found {:?}", other)),
        }
    }

    fn current(&self) -> TokenKind {
        self.tokens
            .get(self.pos)
            .map(|token| token.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn current_token(&self) -> Token {
        self.tokens
            .get(self.pos)
            .cloned()
            .unwrap_or(Token::new(TokenKind::Eof, 0, 0))
    }

    fn peek_token(&self) -> TokenKind {
        self.tokens
            .get(self.pos + 1)
            .map(|token| token.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.current_token();

        if !self.is_at_end() {
            self.pos += 1;
        }

        token
    }

    fn expect(&mut self, expected: TokenKind) -> Result<(), String> {
        let actual = self.advance();

        if actual.kind == expected {
            Ok(())
        } else {
            Err(format!(
                "line {}, column {}: Expected {:?}, found {:?}",
                actual.line, actual.column, expected, actual.kind
            ))
        }
    }

    fn check(&self, expected: &TokenKind) -> bool {
        &self.current() == expected
    }

    fn matches(&mut self, expected: &TokenKind) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn error<T>(&self, message: String) -> Result<T, String> {
        let token = self.current_token();

        Err(format!(
            "line {}, column {}: {}",
            token.line, token.column, message
        ))
    }

    fn is_at_end(&self) -> bool {
        self.current() == TokenKind::Eof
    }
}