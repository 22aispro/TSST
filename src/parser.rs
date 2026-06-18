use crate::ast::{
    Assignment, BinaryOp, Expr, ForEachStmt, ForStmt, Function, FunctionCall, IfStmt, Item,
    MacroCall, Param, Program, ReturnStmt, Stmt, UnaryOp, VarDecl, WhileStmt,
};
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();

        while !self.is_at_end() {
            if self.match_kind(&TokenKind::Use) {
                self.parse_use_line()?;
                continue;
            }

            let is_pub = self.match_kind(&TokenKind::Pub);

            if self.match_kind(&TokenKind::Fcn) {
                items.push(Item::Function(self.parse_function(is_pub)?));
                continue;
            }

            if is_pub {
                return self.error_here("Expected function after pub.");
            }

            if self.check_type() {
                items.push(Item::VarDecl(self.parse_var_decl()?));
                continue;
            }

            return self.error_here("Expected function or variable declaration.");
        }

        Ok(Program { items })
    }

    fn parse_use_line(&mut self) -> Result<(), String> {
        self.consume_string("Expected string path after use.")?;
        self.consume(&TokenKind::Semicolon, "Expected ';' after use import.")?;
        Ok(())
    }

    fn parse_function(&mut self, is_pub: bool) -> Result<Function, String> {
        let name = self.consume_ident("Expected function name.")?;

        self.consume(&TokenKind::LParen, "Expected '(' after function name.")?;

        let mut params = Vec::new();

        if !self.check(&TokenKind::RParen) {
            loop {
                let ty = self.consume_type("Expected parameter type.")?;
                let name = self.consume_ident("Expected parameter name.")?;
                params.push(Param { ty, name });

                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }

                if self.check(&TokenKind::RParen) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RParen, "Expected ')' after parameters.")?;

        let return_type = if self.match_kind(&TokenKind::Arrow) {
            Some(self.consume_return_type("Expected return type after '->'.")?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Function {
            is_pub,
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        if self.check_type() {
            return Ok(Stmt::VarDecl(self.parse_var_decl()?));
        }

        if self.match_kind(&TokenKind::If) {
            return Ok(Stmt::If(self.parse_if()?));
        }

        if self.match_kind(&TokenKind::While) {
            return Ok(Stmt::While(self.parse_while()?));
        }

        if self.match_kind(&TokenKind::For) {
            return self.parse_for_stmt();
        }

        if self.match_kind(&TokenKind::Break) {
            self.consume(&TokenKind::Semicolon, "Expected ';' after break.")?;
            return Ok(Stmt::Break);
        }

        if self.match_kind(&TokenKind::Continue) {
            self.consume(&TokenKind::Semicolon, "Expected ';' after continue.")?;
            return Ok(Stmt::Continue);
        }

        if self.match_kind(&TokenKind::Return) {
            let value = self.parse_expr()?;
            self.consume(&TokenKind::Semicolon, "Expected ';' after return value.")?;
            return Ok(Stmt::Return(ReturnStmt { value }));
        }

        if let Some(name) = self.match_ident() {
            if self.match_kind(&TokenKind::Bang) {
                let args = self.parse_call_args_after_name()?;
                self.consume(&TokenKind::Semicolon, "Expected ';' after macro call.")?;
                return Ok(Stmt::MacroCall(MacroCall { name, args }));
            }

            if self.match_kind(&TokenKind::LParen) {
                let args = self.parse_args_inside_parens()?;
                self.consume(&TokenKind::Semicolon, "Expected ';' after function call.")?;
                return Ok(Stmt::FunctionCall(FunctionCall { name, args }));
            }

            if self.match_kind(&TokenKind::Equal) {
                let value = self.parse_expr()?;
                self.consume(&TokenKind::Semicolon, "Expected ';' after assignment.")?;
                return Ok(Stmt::Assignment(Assignment { name, value }));
            }

            return self.error_here("Expected macro call, function call, or assignment.");
        }

        self.error_here("Expected statement.")
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, String> {
        let ty = self.consume_type("Expected variable type.")?;
        let name = self.consume_ident("Expected variable name.")?;

        self.consume(&TokenKind::Equal, "Expected '=' after variable name.")?;

        let value = self.parse_expr()?;

        self.consume(&TokenKind::Semicolon, "Expected ';' after variable declaration.")?;

        Ok(VarDecl { ty, name, value })
    }

    fn parse_if(&mut self) -> Result<IfStmt, String> {
        let condition = self.parse_expr()?;
        let then_body = self.parse_block()?;

        let else_body = if self.match_kind(&TokenKind::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(IfStmt {
            condition,
            then_body,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<WhileStmt, String> {
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;

        Ok(WhileStmt { condition, body })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(&TokenKind::LParen, "Expected '(' after for.")?;

        let item_ty = self.consume_type("Expected for-loop variable type.")?;
        let item_name = self.consume_ident("Expected for-loop variable name.")?;

        if self.match_kind(&TokenKind::In) {
            let iterable = self.parse_expr()?;
            self.consume(&TokenKind::RParen, "Expected ')' after for-each iterable.")?;
            let body = self.parse_block()?;

            return Ok(Stmt::ForEach(ForEachStmt {
                item_ty,
                item_name,
                iterable,
                body,
            }));
        }

        self.consume(&TokenKind::Equal, "Expected '=' in for-loop initializer.")?;

        let init_value = self.parse_expr()?;

        let initializer = VarDecl {
            ty: item_ty,
            name: item_name,
            value: init_value,
        };

        self.consume(&TokenKind::Semicolon, "Expected ';' after for-loop initializer.")?;

        let condition = self.parse_expr()?;

        self.consume(&TokenKind::Semicolon, "Expected ';' after for-loop condition.")?;

        let update_name = self.consume_ident("Expected update variable name.")?;
        self.consume(&TokenKind::Equal, "Expected '=' in for-loop update.")?;
        let update_value = self.parse_expr()?;

        let update = Assignment {
            name: update_name,
            value: update_value,
        };

        self.consume(&TokenKind::RParen, "Expected ')' after for-loop update.")?;

        let body = self.parse_block()?;

        Ok(Stmt::For(ForStmt {
            initializer,
            condition,
            update,
            body,
        }))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.consume(&TokenKind::LBrace, "Expected '{' before block.")?;

        let mut body = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
        }

        self.consume(&TokenKind::RBrace, "Expected '}' after block.")?;

        Ok(body)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_and()?;

        while self.match_kind(&TokenKind::OrOr) {
            let right = self.parse_and()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_equality()?;

        while self.match_kind(&TokenKind::AndAnd) {
            let right = self.parse_equality()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_comparison()?;

        loop {
            let op = if self.match_kind(&TokenKind::EqualEqual) {
                Some(BinaryOp::Eq)
            } else if self.match_kind(&TokenKind::BangEqual) {
                Some(BinaryOp::NotEq)
            } else {
                None
            };

            match op {
                Some(op) => {
                    let right = self.parse_comparison()?;

                    expr = Expr::Binary {
                        left: Box::new(expr),
                        op,
                        right: Box::new(right),
                    };
                }

                None => break,
            }
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_term()?;

        loop {
            let op = if self.match_kind(&TokenKind::Less) {
                Some(BinaryOp::Less)
            } else if self.match_kind(&TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.match_kind(&TokenKind::LessEqual) {
                Some(BinaryOp::LessEq)
            } else if self.match_kind(&TokenKind::GreaterEqual) {
                Some(BinaryOp::GreaterEq)
            } else {
                None
            };

            match op {
                Some(op) => {
                    let right = self.parse_term()?;

                    expr = Expr::Binary {
                        left: Box::new(expr),
                        op,
                        right: Box::new(right),
                    };
                }

                None => break,
            }
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_factor()?;

        loop {
            let op = if self.match_kind(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.match_kind(&TokenKind::Minus) {
                Some(BinaryOp::Sub)
            } else {
                None
            };

            match op {
                Some(op) => {
                    let right = self.parse_factor()?;

                    expr = Expr::Binary {
                        left: Box::new(expr),
                        op,
                        right: Box::new(right),
                    };
                }

                None => break,
            }
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = if self.match_kind(&TokenKind::Star) {
                Some(BinaryOp::Mul)
            } else if self.match_kind(&TokenKind::Slash) {
                Some(BinaryOp::Div)
            } else {
                None
            };

            match op {
                Some(op) => {
                    let right = self.parse_unary()?;

                    expr = Expr::Binary {
                        left: Box::new(expr),
                        op,
                        right: Box::new(right),
                    };
                }

                None => break,
            }
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.match_kind(&TokenKind::Bang) {
            let expr = self.parse_unary()?;

            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }

        if self.match_kind(&TokenKind::Minus) {
            let expr = self.parse_unary()?;

            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_kind(&TokenKind::LParen) {
                let name = match expr {
                    Expr::Ident(name) => name,
                    _ => return self.error_here("Only named functions can be called."),
                };

                let args = self.parse_args_inside_parens()?;

                expr = Expr::Call { name, args };

                continue;
            }

            if self.match_kind(&TokenKind::LBracket) {
                let index = self.parse_expr()?;
                self.consume(&TokenKind::RBracket, "Expected ']' after index.")?;

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
        if let TokenKind::Int(value) = self.current_kind().clone() {
            self.advance();
            return Ok(Expr::Int(value));
        }

        if let TokenKind::Str(value) = self.current_kind().clone() {
            self.advance();
            return Ok(Expr::Str(value));
        }

        if let TokenKind::Bool(value) = self.current_kind().clone() {
            self.advance();
            return Ok(Expr::Bool(value));
        }

        if let TokenKind::Ident(name) = self.current_kind().clone() {
            self.advance();
            return Ok(Expr::Ident(name));
        }

        if self.match_kind(&TokenKind::LParen) {
            let expr = self.parse_expr()?;
            self.consume(&TokenKind::RParen, "Expected ')' after expression.")?;
            return Ok(expr);
        }

        if self.match_kind(&TokenKind::LBracket) {
            return self.parse_array_literal();
        }

        if self.match_kind(&TokenKind::LBrace) {
            return self.parse_dict_literal();
        }

        self.error_here(&format!(
            "Expected expression, found {}.",
            self.describe_current()
        ))
    }

    fn parse_array_literal(&mut self) -> Result<Expr, String> {
        let mut values = Vec::new();

        if !self.check(&TokenKind::RBracket) {
            loop {
                values.push(self.parse_expr()?);

                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }

                if self.check(&TokenKind::RBracket) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RBracket, "Expected ']' after array.")?;

        Ok(Expr::ArrayLiteral(values))
    }

    fn parse_dict_literal(&mut self) -> Result<Expr, String> {
        let mut values = Vec::new();

        if !self.check(&TokenKind::RBrace) {
            loop {
                let key = self.consume_string("Expected dictionary string key.")?;
                self.consume(&TokenKind::Colon, "Expected ':' after dictionary key.")?;
                let value = self.parse_expr()?;

                values.push((key, value));

                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }

                if self.check(&TokenKind::RBrace) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RBrace, "Expected '}' after dictionary.")?;

        Ok(Expr::DictLiteral(values))
    }

    fn parse_call_args_after_name(&mut self) -> Result<Vec<Expr>, String> {
        self.consume(&TokenKind::LParen, "Expected '(' after call name.")?;
        self.parse_args_inside_parens()
    }

    fn parse_args_inside_parens(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();

        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);

                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }

                if self.check(&TokenKind::RParen) {
                    break;
                }
            }
        }

        self.consume(&TokenKind::RParen, "Expected ')' after arguments.")?;

        Ok(args)
    }

    fn consume(&mut self, expected: &TokenKind, message: &str) -> Result<(), String> {
        if self.check(expected) {
            self.advance();
            Ok(())
        } else {
            self.error_here(message)
        }
    }

    fn consume_ident(&mut self, message: &str) -> Result<String, String> {
        match self.current_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            _ => self.error_here(message),
        }
    }

    fn consume_string(&mut self, message: &str) -> Result<String, String> {
        match self.current_kind().clone() {
            TokenKind::Str(value) => {
                self.advance();
                Ok(value)
            }
            _ => self.error_here(message),
        }
    }

    fn consume_type(&mut self, message: &str) -> Result<String, String> {
        match self.current_kind().clone() {
            TokenKind::Type(value) => {
                self.advance();
                Ok(value)
            }
            _ => self.error_here(message),
        }
    }

    fn consume_return_type(&mut self, message: &str) -> Result<String, String> {
        match self.current_kind().clone() {
            TokenKind::Type(value) => {
                self.advance();
                Ok(value)
            }

            TokenKind::Ident(value) => {
                self.advance();
                Ok(value)
            }

            _ => self.error_here(message),
        }
    }

    fn match_ident(&mut self) -> Option<String> {
        match self.current_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Some(name)
            }
            _ => None,
        }
    }

    fn match_kind(&mut self, expected: &TokenKind) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, expected: &TokenKind) -> bool {
        if self.is_at_end() {
            return matches!(expected, TokenKind::Eof);
        }

        same_kind(self.current_kind(), expected)
    }

    fn check_type(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Type(_))
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }

        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    fn current_kind(&self) -> &TokenKind {
        &self.tokens[self.current].kind
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn error_here<T>(&self, message: &str) -> Result<T, String> {
        let token = self.current_token();

        Err(format!(
            "line {}, column {}: {}",
            token.line, token.column, message
        ))
    }

    fn describe_current(&self) -> String {
        match self.current_kind() {
            TokenKind::Eof => "EOF".to_string(),
            TokenKind::Ident(value) => format!("identifier '{}'", value),
            TokenKind::Int(value) => format!("integer '{}'", value),
            TokenKind::Str(value) => format!("string \"{}\"", value),
            TokenKind::Bool(value) => format!("boolean '{}'", value),
            TokenKind::Type(value) => format!("type '{}'", value),
            other => format!("{:?}", other),
        }
    }
}

fn same_kind(left: &TokenKind, right: &TokenKind) -> bool {
    match (left, right) {
        (TokenKind::Ident(_), TokenKind::Ident(_)) => true,
        (TokenKind::Int(_), TokenKind::Int(_)) => true,
        (TokenKind::Str(_), TokenKind::Str(_)) => true,
        (TokenKind::Bool(_), TokenKind::Bool(_)) => true,
        (TokenKind::Type(_), TokenKind::Type(_)) => true,
        _ => left == right,
    }
}