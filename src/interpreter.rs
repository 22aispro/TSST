use std::collections::HashMap;

use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
}

impl Value {
    fn to_output(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Str(value) => value.clone(),
            Value::Bool(value) => value.to_string(),
        }
    }
}

pub struct Interpreter {
    variables: HashMap<String, Value>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), String> {
        let main = program
            .items
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name == "main" => Some(function),
                _ => None,
            })
            .ok_or("No main function found")?;

        self.run_function(main)
    }

    fn run_function(&mut self, function: &Function) -> Result<(), String> {
        for stmt in &function.body {
            self.run_stmt(stmt)?;
        }

        Ok(())
    }

    fn run_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl(var_decl) => self.run_var_decl(var_decl),
            Stmt::MacroCall(call) => self.run_macro_call(call),
            Stmt::Assignment(assignment) => self.run_assignment(assignment),
            Stmt::If(if_stmt) => self.run_if(if_stmt),
        }
    }

    fn run_var_decl(&mut self, var_decl: &VarDecl) -> Result<(), String> {
        let value = self.eval_expr(&var_decl.value)?;

        match (var_decl.ty.as_str(), &value) {
            ("int", Value::Int(_)) => {}
            ("str", Value::Str(_)) => {}
            ("bool", Value::Bool(_)) => {}

            (expected, actual) => {
                return Err(format!(
                    "Type error: variable '{}' expected {}, got {:?}",
                    var_decl.name, expected, actual
                ));
            }
        }

        self.variables.insert(var_decl.name.clone(), value);

        Ok(())
    }

    fn run_assignment(&mut self, assignment: &Assignment) -> Result<(), String> {
        if !self.variables.contains_key(&assignment.name) {
            return Err(format!("Unknown variable: {}", assignment.name));
        }

        let old_value = self
            .variables
            .get(&assignment.name)
            .cloned()
            .ok_or(format!("Unknown variable: {}", assignment.name))?;

        let new_value = self.eval_expr(&assignment.value)?;

        match (&old_value, &new_value) {
            (Value::Int(_), Value::Int(_)) => {}
            (Value::Str(_), Value::Str(_)) => {}
            (Value::Bool(_), Value::Bool(_)) => {}

            (old, new) => {
                return Err(format!(
                    "Type error: cannot assign {:?} to variable '{}' with old value {:?}",
                    new, assignment.name, old
                ));
            }
        }

        self.variables.insert(assignment.name.clone(), new_value);

        Ok(())
    }

    fn run_if(&mut self, if_stmt: &IfStmt) -> Result<(), String> {
        let condition = self.eval_expr(&if_stmt.condition)?;

        match condition {
            Value::Bool(true) => {
                for stmt in &if_stmt.then_body {
                    self.run_stmt(stmt)?;
                }

                Ok(())
            }

            Value::Bool(false) => {
                if let Some(else_body) = &if_stmt.else_body {
                    for stmt in else_body {
                        self.run_stmt(stmt)?;
                    }
                }

                Ok(())
            }

            other => Err(format!("If condition must be bool, got {:?}", other)),
        }
    }

    fn run_macro_call(&mut self, call: &MacroCall) -> Result<(), String> {
        match call.name.as_str() {
            "cons" => {
                for arg in &call.args {
                    let value = self.eval_expr(arg)?;
                    println!("{}", value.to_output());
                }

                Ok(())
            }

            other => Err(format!("Unknown macro: {}!", other)),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Int(value) => Ok(Value::Int(*value)),

            Expr::Str(value) => Ok(Value::Str(value.clone())),

            Expr::Bool(value) => Ok(Value::Bool(*value)),

            Expr::Ident(name) => self
                .variables
                .get(name)
                .cloned()
                .ok_or(format!("Unknown variable: {}", name)),

            Expr::Binary { left, op, right } => {
                let left = self.eval_expr(left)?;
                let right = self.eval_expr(right)?;

                self.eval_binary(left, op, right)
            }
        }
    }

    fn eval_binary(&mut self, left: Value, op: &BinaryOp, right: Value) -> Result<Value, String> {
        match (left, op, right) {
            (Value::Int(a), BinaryOp::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), BinaryOp::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), BinaryOp::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),

            (Value::Int(_), BinaryOp::Div, Value::Int(0)) => {
                Err("Runtime error: division by zero".to_string())
            }

            (Value::Int(a), BinaryOp::Div, Value::Int(b)) => Ok(Value::Int(a / b)),

            (Value::Int(a), BinaryOp::Less, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), BinaryOp::Greater, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), BinaryOp::LessEq, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), BinaryOp::GreaterEq, Value::Int(b)) => Ok(Value::Bool(a >= b)),

            (a, BinaryOp::Eq, b) => Ok(Value::Bool(a == b)),
            (a, BinaryOp::NotEq, b) => Ok(Value::Bool(a != b)),

            (left, op, right) => Err(format!(
                "Type error: cannot apply {:?} to {:?} and {:?}",
                op, left, right
            )),
        }
    }
}