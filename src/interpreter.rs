use std::collections::HashMap;

use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
}

enum ExecSignal {
    None,
    Return(Value),
    Break,
    Continue,
}

impl Value {
    fn to_output(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Str(value) => value.clone(),
            Value::Bool(value) => value.to_string(),

            Value::Array(values) => {
                let parts: Vec<String> = values.iter().map(|value| value.to_output()).collect();
                format!("[{}]", parts.join(", "))
            }

            Value::Dict(values) => {
                let mut parts = Vec::new();
                let mut keys: Vec<String> = values.keys().cloned().collect();

                keys.sort();

                for key in keys {
                    if let Some(value) = values.get(&key) {
                        parts.push(format!("{}: {}", key, value.to_output()));
                    }
                }

                format!("{{{}}}", parts.join(", "))
            }
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Str(_) => "str",
            Value::Bool(_) => "bool",
            Value::Array(_) => "arr",
            Value::Dict(_) => "dict",
        }
    }
}

pub struct Interpreter {
    scopes: Vec<HashMap<String, Value>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), String> {
        self.call_function_by_name(program, "main", Vec::new())?;
        Ok(())
    }

    fn call_function_by_name(
        &mut self,
        program: &Program,
        name: &str,
        arg_values: Vec<Value>,
    ) -> Result<Option<Value>, String> {
        if name == "len" {
            return self.call_builtin_len(arg_values);
        }

        let function = program
            .items
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name == name => Some(function),
                _ => None,
            })
            .ok_or(format!("No function named '{}' found", name))?;

        if function.params.len() != arg_values.len() {
            return Err(format!(
                "Function '{}' expected {} args, got {}",
                name,
                function.params.len(),
                arg_values.len()
            ));
        }

        let mut scope = HashMap::new();

        for (param, value) in function.params.iter().zip(arg_values.into_iter()) {
            if !Self::type_matches(&param.ty, &value) {
                return Err(format!(
                    "Type error: parameter '{}' expected {}, got {}",
                    param.name,
                    param.ty,
                    value.type_name()
                ));
            }

            scope.insert(param.name.clone(), value);
        }

        self.scopes.push(scope);

        let result = (|| -> Result<Option<Value>, String> {
            for stmt in &function.body {
                match self.run_stmt(program, stmt)? {
                    ExecSignal::None => {}
                    ExecSignal::Return(value) => return Ok(Some(value)),
                    ExecSignal::Break => return Err("break used outside of loop".to_string()),
                    ExecSignal::Continue => return Err("continue used outside of loop".to_string()),
                }
            }

            Ok(None)
        })();

        self.scopes.pop();

        let returned = result?;

        match (&function.return_type, returned) {
            (Some(expected), Some(value)) => {
                if !Self::type_matches(expected, &value) {
                    return Err(format!(
                        "Function '{}' should return {}, got {}",
                        name,
                        expected,
                        value.type_name()
                    ));
                }

                Ok(Some(value))
            }

            (Some(expected), None) => Err(format!(
                "Function '{}' should return {}, but returned nothing",
                name, expected
            )),

            (None, Some(_)) => Err(format!(
                "Function '{}' returned a value but has no return type",
                name
            )),

            (None, None) => Ok(None),
        }
    }

    fn call_builtin_len(&mut self, arg_values: Vec<Value>) -> Result<Option<Value>, String> {
        if arg_values.len() != 1 {
            return Err(format!("len expected 1 arg, got {}", arg_values.len()));
        }

        let value = arg_values.into_iter().next().unwrap();

        let len = match value {
            Value::Str(value) => value.chars().count() as i64,
            Value::Array(values) => values.len() as i64,
            Value::Dict(values) => values.len() as i64,

            other => {
                return Err(format!("len cannot be used on {}", other.type_name()));
            }
        };

        Ok(Some(Value::Int(len)))
    }

    fn run_stmt(&mut self, program: &Program, stmt: &Stmt) -> Result<ExecSignal, String> {
        match stmt {
            Stmt::VarDecl(var_decl) => {
                self.run_var_decl(program, var_decl)?;
                Ok(ExecSignal::None)
            }

            Stmt::MacroCall(call) => {
                self.run_macro_call(program, call)?;
                Ok(ExecSignal::None)
            }

            Stmt::FunctionCall(call) => {
                let args = self.eval_args(program, &call.args)?;
                self.call_function_by_name(program, &call.name, args)?;
                Ok(ExecSignal::None)
            }

            Stmt::Assignment(assignment) => {
                self.run_assignment(program, assignment)?;
                Ok(ExecSignal::None)
            }

            Stmt::If(if_stmt) => self.run_if(program, if_stmt),

            Stmt::While(while_stmt) => self.run_while(program, while_stmt),

            Stmt::For(for_stmt) => self.run_for(program, for_stmt),

            Stmt::ForEach(for_each_stmt) => self.run_for_each(program, for_each_stmt),

            Stmt::Break => Ok(ExecSignal::Break),

            Stmt::Continue => Ok(ExecSignal::Continue),

            Stmt::Return(return_stmt) => {
                let value = self.eval_expr(program, &return_stmt.value)?;
                Ok(ExecSignal::Return(value))
            }
        }
    }

    fn run_var_decl(&mut self, program: &Program, var_decl: &VarDecl) -> Result<(), String> {
        let value = self.eval_expr(program, &var_decl.value)?;

        if !Self::type_matches(&var_decl.ty, &value) {
            return Err(format!(
                "Type error: variable '{}' expected {}, got {}",
                var_decl.name,
                var_decl.ty,
                value.type_name()
            ));
        }

        self.define_var(var_decl.name.clone(), value);

        Ok(())
    }

    fn run_assignment(&mut self, program: &Program, assignment: &Assignment) -> Result<(), String> {
        let old_value = self
            .get_var(&assignment.name)
            .ok_or(format!("Unknown variable: {}", assignment.name))?;

        let new_value = self.eval_expr(program, &assignment.value)?;

        if old_value.type_name() != new_value.type_name() {
            return Err(format!(
                "Type error: cannot assign {} to variable '{}' of type {}",
                new_value.type_name(),
                assignment.name,
                old_value.type_name()
            ));
        }

        self.assign_var(&assignment.name, new_value)
    }

    fn run_if(&mut self, program: &Program, if_stmt: &IfStmt) -> Result<ExecSignal, String> {
        let condition = self.eval_expr(program, &if_stmt.condition)?;

        match condition {
            Value::Bool(true) => {
                for stmt in &if_stmt.then_body {
                    match self.run_stmt(program, stmt)? {
                        ExecSignal::None => {}
                        signal => return Ok(signal),
                    }
                }

                Ok(ExecSignal::None)
            }

            Value::Bool(false) => {
                if let Some(else_body) = &if_stmt.else_body {
                    for stmt in else_body {
                        match self.run_stmt(program, stmt)? {
                            ExecSignal::None => {}
                            signal => return Ok(signal),
                        }
                    }
                }

                Ok(ExecSignal::None)
            }

            other => Err(format!("If condition must be bool, got {:?}", other)),
        }
    }

    fn run_while(&mut self, program: &Program, while_stmt: &WhileStmt) -> Result<ExecSignal, String> {
        loop {
            let condition = self.eval_expr(program, &while_stmt.condition)?;

            match condition {
                Value::Bool(true) => {
                    for stmt in &while_stmt.body {
                        match self.run_stmt(program, stmt)? {
                            ExecSignal::None => {}
                            ExecSignal::Continue => break,
                            ExecSignal::Break => return Ok(ExecSignal::None),
                            signal @ ExecSignal::Return(_) => return Ok(signal),
                        }
                    }
                }

                Value::Bool(false) => break,

                other => {
                    return Err(format!("While condition must be bool, got {:?}", other));
                }
            }
        }

        Ok(ExecSignal::None)
    }

    fn run_for(&mut self, program: &Program, for_stmt: &ForStmt) -> Result<ExecSignal, String> {
        self.scopes.push(HashMap::new());

        let result = (|| -> Result<ExecSignal, String> {
            self.run_var_decl(program, &for_stmt.initializer)?;

            loop {
                let condition = self.eval_expr(program, &for_stmt.condition)?;

                match condition {
                    Value::Bool(true) => {
                        for stmt in &for_stmt.body {
                            match self.run_stmt(program, stmt)? {
                                ExecSignal::None => {}
                                ExecSignal::Continue => break,
                                ExecSignal::Break => return Ok(ExecSignal::None),
                                signal @ ExecSignal::Return(_) => return Ok(signal),
                            }
                        }

                        self.run_assignment(program, &for_stmt.update)?;
                    }

                    Value::Bool(false) => break,

                    other => {
                        return Err(format!("For condition must be bool, got {:?}", other));
                    }
                }
            }

            Ok(ExecSignal::None)
        })();

        self.scopes.pop();

        result
    }

    fn run_for_each(
        &mut self,
        program: &Program,
        for_each_stmt: &ForEachStmt,
    ) -> Result<ExecSignal, String> {
        let iterable = self.eval_expr(program, &for_each_stmt.iterable)?;

        let values = match iterable {
            Value::Array(values) => values,

            Value::Dict(values) => {
                let mut keys: Vec<String> = values.keys().cloned().collect();
                keys.sort();
                keys.into_iter().map(Value::Str).collect()
            }

            other => {
                return Err(format!(
                    "For-each loop expected array or dict, got {}",
                    other.type_name()
                ));
            }
        };

        self.scopes.push(HashMap::new());

        let result = (|| -> Result<ExecSignal, String> {
            for value in values {
                if !Self::type_matches(&for_each_stmt.item_ty, &value) {
                    return Err(format!(
                        "Type error: for-each variable '{}' expected {}, got {}",
                        for_each_stmt.item_name,
                        for_each_stmt.item_ty,
                        value.type_name()
                    ));
                }

                self.define_var(for_each_stmt.item_name.clone(), value);

                for stmt in &for_each_stmt.body {
                    match self.run_stmt(program, stmt)? {
                        ExecSignal::None => {}
                        ExecSignal::Continue => break,
                        ExecSignal::Break => return Ok(ExecSignal::None),
                        signal @ ExecSignal::Return(_) => return Ok(signal),
                    }
                }
            }

            Ok(ExecSignal::None)
        })();

        self.scopes.pop();

        result
    }

    fn run_macro_call(&mut self, program: &Program, call: &MacroCall) -> Result<(), String> {
        match call.name.as_str() {
            "cons" => {
                for arg in &call.args {
                    let value = self.eval_expr(program, arg)?;
                    println!("{}", value.to_output());
                }

                Ok(())
            }

            "push" => self.run_push_macro(program, call),

            "set" => self.run_set_macro(program, call),

            other => Err(format!("Unknown macro: {}!", other)),
        }
    }

    fn run_push_macro(&mut self, program: &Program, call: &MacroCall) -> Result<(), String> {
        if call.args.len() != 2 {
            return Err(format!("push! expected 2 args, got {}", call.args.len()));
        }

        let array_name = match &call.args[0] {
            Expr::Ident(name) => name.clone(),
            _ => return Err("push! first argument must be an array variable name".to_string()),
        };

        let value = self.eval_expr(program, &call.args[1])?;

        let array = self
            .get_var_mut(&array_name)
            .ok_or(format!("Unknown variable: {}", array_name))?;

        match array {
            Value::Array(values) => {
                values.push(value);
                Ok(())
            }

            other => Err(format!(
                "push! expected array variable, got {}",
                other.type_name()
            )),
        }
    }

    fn run_set_macro(&mut self, program: &Program, call: &MacroCall) -> Result<(), String> {
        if call.args.len() != 3 {
            return Err(format!("set! expected 3 args, got {}", call.args.len()));
        }

        let dict_name = match &call.args[0] {
            Expr::Ident(name) => name.clone(),
            _ => return Err("set! first argument must be a dictionary variable name".to_string()),
        };

        let key = match self.eval_expr(program, &call.args[1])? {
            Value::Str(key) => key,
            other => {
                return Err(format!(
                    "set! dictionary key must be str, got {}",
                    other.type_name()
                ));
            }
        };

        let value = self.eval_expr(program, &call.args[2])?;

        let dict = self
            .get_var_mut(&dict_name)
            .ok_or(format!("Unknown variable: {}", dict_name))?;

        match dict {
            Value::Dict(values) => {
                values.insert(key, value);
                Ok(())
            }

            other => Err(format!(
                "set! expected dictionary variable, got {}",
                other.type_name()
            )),
        }
    }

    fn eval_expr(&mut self, program: &Program, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Int(value) => Ok(Value::Int(*value)),

            Expr::Str(value) => Ok(Value::Str(value.clone())),

            Expr::Bool(value) => Ok(Value::Bool(*value)),

            Expr::Ident(name) => self
                .get_var(name)
                .ok_or(format!("Unknown variable: {}", name)),

            Expr::ArrayLiteral(items) => {
                let mut values = Vec::new();

                for item in items {
                    values.push(self.eval_expr(program, item)?);
                }

                Ok(Value::Array(values))
            }

            Expr::DictLiteral(pairs) => {
                let mut values = HashMap::new();

                for (key, value_expr) in pairs {
                    let value = self.eval_expr(program, value_expr)?;
                    values.insert(key.clone(), value);
                }

                Ok(Value::Dict(values))
            }

            Expr::Index { target, index } => {
                let target = self.eval_expr(program, target)?;
                let index = self.eval_expr(program, index)?;

                match (target, index) {
                    (Value::Array(values), Value::Int(index)) => {
                        if index < 0 {
                            return Err(format!("Array index cannot be negative: {}", index));
                        }

                        let index = index as usize;

                        values
                            .get(index)
                            .cloned()
                            .ok_or(format!("Array index out of bounds: {}", index))
                    }

                    (Value::Dict(values), Value::Str(key)) => values
                        .get(&key)
                        .cloned()
                        .ok_or(format!("Dictionary key not found: {}", key)),

                    (target, index) => Err(format!(
                        "Cannot index {} with {}",
                        target.type_name(),
                        index.type_name()
                    )),
                }
            }

            Expr::Call { name, args } => {
                let arg_values = self.eval_args(program, args)?;

                match self.call_function_by_name(program, name, arg_values)? {
                    Some(value) => Ok(value),
                    None => Err(format!("Function '{}' does not return a value", name)),
                }
            }

            Expr::Binary { left, op, right } => {
                let left = self.eval_expr(program, left)?;
                let right = self.eval_expr(program, right)?;

                self.eval_binary(left, op, right)
            }
        }
    }

    fn eval_args(&mut self, program: &Program, args: &[Expr]) -> Result<Vec<Value>, String> {
        let mut values = Vec::new();

        for arg in args {
            values.push(self.eval_expr(program, arg)?);
        }

        Ok(values)
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

            (Value::Str(a), BinaryOp::Add, Value::Str(b)) => {
                Ok(Value::Str(format!("{}{}", a, b)))
            }

            (Value::Str(a), BinaryOp::Add, b) => {
                Ok(Value::Str(format!("{}{}", a, b.to_output())))
            }

            (a, BinaryOp::Add, Value::Str(b)) => {
                Ok(Value::Str(format!("{}{}", a.to_output(), b)))
            }

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

    fn define_var(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    fn get_var(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }

        None
    }

    fn get_var_mut(&mut self, name: &str) -> Option<&mut Value> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                return scope.get_mut(name);
            }
        }

        None
    }

    fn assign_var(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }

        Err(format!("Unknown variable: {}", name))
    }

    fn type_matches(expected: &str, value: &Value) -> bool {
        expected == value.type_name()
    }
}