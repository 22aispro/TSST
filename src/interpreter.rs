use crate::ast::{
    Assignment, BinaryOp, Expr, ForEachStmt, ForStmt, Function, FunctionCall, IfStmt, Item,
    MacroCall, Program, ReturnStmt, Stmt, UnaryOp, VarDecl, WhileStmt,
};

use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Str(_) => "str",
            Value::Bool(_) => "bool",
            Value::Array(_) => "arr",
            Value::Dict(_) => "dict",
        }
    }

    pub fn to_output(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Str(value) => value.clone(),
            Value::Bool(value) => value.to_string(),
            Value::Array(values) => {
                let parts: Vec<String> = values.iter().map(|value| value.to_output()).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Dict(values) => {
                let mut keys: Vec<String> = values.keys().cloned().collect();
                keys.sort();

                let parts: Vec<String> = keys
                    .iter()
                    .map(|key| {
                        let value = values.get(key).unwrap();
                        format!("{}: {}", key, value.to_output())
                    })
                    .collect();

                format!("{{{}}}", parts.join(", "))
            }
        }
    }
}

#[derive(Debug)]
enum ExecSignal {
    None,
    Return(Value),
    Break,
    Continue,
}

pub struct Interpreter {
    scopes: Vec<HashMap<String, Value>>,
    functions: HashMap<String, usize>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), String> {
        self.functions.clear();

        for (index, item) in program.items.iter().enumerate() {
            if let Item::Function(function) = item {
                self.functions.insert(function.name.clone(), index);
            }
        }

        for item in &program.items {
            if let Item::VarDecl(var_decl) = item {
                self.run_var_decl(var_decl, program)?;
            }
        }

        if self.functions.contains_key("main") {
            self.call_function_by_name("main", Vec::new(), program)?;
        }

        Ok(())
    }

    fn execute_stmt(&mut self, stmt: &Stmt, program: &Program) -> Result<ExecSignal, String> {
        match stmt {
            Stmt::VarDecl(var_decl) => {
                self.run_var_decl(var_decl, program)?;
                Ok(ExecSignal::None)
            }

            Stmt::MacroCall(macro_call) => {
                self.run_macro_call(macro_call, program)?;
                Ok(ExecSignal::None)
            }

            Stmt::FunctionCall(function_call) => {
                self.run_function_call(function_call, program)?;
                Ok(ExecSignal::None)
            }

            Stmt::Assignment(assignment) => {
                self.run_assignment(assignment, program)?;
                Ok(ExecSignal::None)
            }

            Stmt::If(if_stmt) => self.run_if(if_stmt, program),

            Stmt::While(while_stmt) => self.run_while(while_stmt, program),

            Stmt::For(for_stmt) => self.run_for(for_stmt, program),

            Stmt::ForEach(for_each_stmt) => self.run_for_each(for_each_stmt, program),

            Stmt::Break => Ok(ExecSignal::Break),

            Stmt::Continue => Ok(ExecSignal::Continue),

            Stmt::Return(return_stmt) => self.run_return(return_stmt, program),
        }
    }

    fn execute_block(&mut self, body: &[Stmt], program: &Program) -> Result<ExecSignal, String> {
        for stmt in body {
            let signal = self.execute_stmt(stmt, program)?;

            match signal {
                ExecSignal::None => {}
                ExecSignal::Return(_) | ExecSignal::Break | ExecSignal::Continue => {
                    return Ok(signal);
                }
            }
        }

        Ok(ExecSignal::None)
    }

    fn run_var_decl(&mut self, var_decl: &VarDecl, program: &Program) -> Result<(), String> {
        let value = self.eval_expr(&var_decl.value, program)?;

        if !self.type_matches(&var_decl.ty, &value) {
            return Err(format!(
                "Type mismatch for '{}'. Expected {}, got {}.",
                var_decl.name,
                self.clean_type(&var_decl.ty),
                value.type_name()
            ));
        }

        self.define_var(var_decl.name.clone(), value);

        Ok(())
    }

    fn run_assignment(&mut self, assignment: &Assignment, program: &Program) -> Result<(), String> {
        let value = self.eval_expr(&assignment.value, program)?;

        let old_value = self
            .get_var(&assignment.name)
            .ok_or_else(|| format!("Undefined variable '{}'.", assignment.name))?;

        if old_value.type_name() != value.type_name() {
            return Err(format!(
                "Type mismatch assigning '{}'. Expected {}, got {}.",
                assignment.name,
                old_value.type_name(),
                value.type_name()
            ));
        }

        self.set_var(&assignment.name, value)?;

        Ok(())
    }

    fn run_macro_call(&mut self, macro_call: &MacroCall, program: &Program) -> Result<(), String> {
        match macro_call.name.as_str() {
            "cons" => {
                if macro_call.args.len() != 1 {
                    return Err("cons!() expects 1 argument.".to_string());
                }

                let value = self.eval_expr(&macro_call.args[0], program)?;
                println!("{}", value.to_output());

                Ok(())
            }

            "push" => {
                if macro_call.args.len() != 2 {
                    return Err("push!() expects 2 arguments.".to_string());
                }

                let target_name = match &macro_call.args[0] {
                    Expr::Ident(name) => name.clone(),
                    _ => {
                        return Err(
                            "push!() first argument must be an array variable.".to_string()
                        );
                    }
                };

                let value = self.eval_expr(&macro_call.args[1], program)?;

                let target = self
                    .get_var_mut(&target_name)
                    .ok_or_else(|| format!("Undefined array variable '{}'.", target_name))?;

                match target {
                    Value::Array(values) => {
                        values.push(value);
                        Ok(())
                    }
                    other => Err(format!("push!() expected arr, got {}.", other.type_name())),
                }
            }

            "set" => {
                if macro_call.args.len() != 3 {
                    return Err("set!() expects 3 arguments.".to_string());
                }

                let target_name = match &macro_call.args[0] {
                    Expr::Ident(name) => name.clone(),
                    _ => {
                        return Err(
                            "set!() first argument must be a dictionary variable.".to_string()
                        );
                    }
                };

                let key = self.eval_expr(&macro_call.args[1], program)?;
                let value = self.eval_expr(&macro_call.args[2], program)?;

                let key_text = match key {
                    Value::Str(value) => value,
                    other => {
                        return Err(format!("set!() key must be str, got {}.", other.type_name()));
                    }
                };

                let target = self
                    .get_var_mut(&target_name)
                    .ok_or_else(|| format!("Undefined dictionary variable '{}'.", target_name))?;

                match target {
                    Value::Dict(values) => {
                        values.insert(key_text, value);
                        Ok(())
                    }
                    other => Err(format!("set!() expected dict, got {}.", other.type_name())),
                }
            }

            other => Err(format!("Unknown macro '{}!'.", other)),
        }
    }

    fn run_function_call(
        &mut self,
        function_call: &FunctionCall,
        program: &Program,
    ) -> Result<(), String> {
        let mut args = Vec::new();

        for arg in &function_call.args {
            args.push(self.eval_expr(arg, program)?);
        }

        self.call_function_by_name(&function_call.name, args, program)?;

        Ok(())
    }

    fn run_if(&mut self, if_stmt: &IfStmt, program: &Program) -> Result<ExecSignal, String> {
        let condition = self.eval_expr(&if_stmt.condition, program)?;

        match condition {
            Value::Bool(true) => self.execute_block(&if_stmt.then_body, program),
            Value::Bool(false) => {
                if let Some(else_body) = &if_stmt.else_body {
                    self.execute_block(else_body, program)
                } else {
                    Ok(ExecSignal::None)
                }
            }
            other => Err(format!(
                "if condition must be bool, got {}.",
                other.type_name()
            )),
        }
    }

    fn run_while(
        &mut self,
        while_stmt: &WhileStmt,
        program: &Program,
    ) -> Result<ExecSignal, String> {
        loop {
            let condition = self.eval_expr(&while_stmt.condition, program)?;

            match condition {
                Value::Bool(true) => {}
                Value::Bool(false) => break,
                other => {
                    return Err(format!(
                        "while condition must be bool, got {}.",
                        other.type_name()
                    ));
                }
            }

            match self.execute_block(&while_stmt.body, program)? {
                ExecSignal::None => {}
                ExecSignal::Break => break,
                ExecSignal::Continue => continue,
                signal @ ExecSignal::Return(_) => return Ok(signal),
            }
        }

        Ok(ExecSignal::None)
    }

    fn run_for(&mut self, for_stmt: &ForStmt, program: &Program) -> Result<ExecSignal, String> {
        self.scopes.push(HashMap::new());

        self.run_var_decl(&for_stmt.initializer, program)?;

        loop {
            let condition = self.eval_expr(&for_stmt.condition, program)?;

            match condition {
                Value::Bool(true) => {}
                Value::Bool(false) => break,
                other => {
                    self.scopes.pop();
                    return Err(format!(
                        "for condition must be bool, got {}.",
                        other.type_name()
                    ));
                }
            }

            match self.execute_block(&for_stmt.body, program)? {
                ExecSignal::None => {}
                ExecSignal::Break => break,
                ExecSignal::Continue => {
                    self.run_assignment(&for_stmt.update, program)?;
                    continue;
                }
                signal @ ExecSignal::Return(_) => {
                    self.scopes.pop();
                    return Ok(signal);
                }
            }

            self.run_assignment(&for_stmt.update, program)?;
        }

        self.scopes.pop();

        Ok(ExecSignal::None)
    }

    fn run_for_each(
        &mut self,
        for_each_stmt: &ForEachStmt,
        program: &Program,
    ) -> Result<ExecSignal, String> {
        let iterable = self.eval_expr(&for_each_stmt.iterable, program)?;

        match iterable {
            Value::Array(values) => {
                for value in values {
                    if !self.type_matches(&for_each_stmt.item_ty, &value) {
                        return Err(format!(
                            "for-each item '{}' expected {}, got {}.",
                            for_each_stmt.item_name,
                            self.clean_type(&for_each_stmt.item_ty),
                            value.type_name()
                        ));
                    }

                    self.scopes.push(HashMap::new());
                    self.define_var(for_each_stmt.item_name.clone(), value);

                    let signal = self.execute_block(&for_each_stmt.body, program)?;

                    self.scopes.pop();

                    match signal {
                        ExecSignal::None => {}
                        ExecSignal::Break => break,
                        ExecSignal::Continue => continue,
                        signal @ ExecSignal::Return(_) => return Ok(signal),
                    }
                }

                Ok(ExecSignal::None)
            }

            Value::Dict(values) => {
                let mut keys: Vec<String> = values.keys().cloned().collect();
                keys.sort();

                for key in keys {
                    let value = Value::Str(key);

                    if !self.type_matches(&for_each_stmt.item_ty, &value) {
                        return Err(format!(
                            "for-each dictionary key '{}' expected {}, got {}.",
                            for_each_stmt.item_name,
                            self.clean_type(&for_each_stmt.item_ty),
                            value.type_name()
                        ));
                    }

                    self.scopes.push(HashMap::new());
                    self.define_var(for_each_stmt.item_name.clone(), value);

                    let signal = self.execute_block(&for_each_stmt.body, program)?;

                    self.scopes.pop();

                    match signal {
                        ExecSignal::None => {}
                        ExecSignal::Break => break,
                        ExecSignal::Continue => continue,
                        signal @ ExecSignal::Return(_) => return Ok(signal),
                    }
                }

                Ok(ExecSignal::None)
            }

            other => Err(format!(
                "for-each expected arr or dict, got {}.",
                other.type_name()
            )),
        }
    }

    fn run_return(
        &mut self,
        return_stmt: &ReturnStmt,
        program: &Program,
    ) -> Result<ExecSignal, String> {
        let value = self.eval_expr(&return_stmt.value, program)?;
        Ok(ExecSignal::Return(value))
    }

    fn eval_expr(&mut self, expr: &Expr, program: &Program) -> Result<Value, String> {
        match expr {
            Expr::Int(value) => Ok(Value::Int(*value)),

            Expr::Str(value) => Ok(Value::Str(value.clone())),

            Expr::Bool(value) => Ok(Value::Bool(*value)),

            Expr::Ident(name) => self
                .get_var(name)
                .ok_or_else(|| format!("Undefined variable '{}'.", name)),

            Expr::ArrayLiteral(values) => {
                let mut result = Vec::new();

                for value in values {
                    result.push(self.eval_expr(value, program)?);
                }

                Ok(Value::Array(result))
            }

            Expr::DictLiteral(values) => {
                let mut result = HashMap::new();

                for (key, value_expr) in values {
                    let value = self.eval_expr(value_expr, program)?;
                    result.insert(key.clone(), value);
                }

                Ok(Value::Dict(result))
            }

            Expr::Index { target, index } => {
                let target_value = self.eval_expr(target, program)?;
                let index_value = self.eval_expr(index, program)?;

                self.eval_index(target_value, index_value)
            }

            Expr::Call { name, args } => {
                let mut values = Vec::new();

                for arg in args {
                    values.push(self.eval_expr(arg, program)?);
                }

                self.call_function_by_name(name, values, program)
            }

            Expr::Unary { op, expr } => {
                let value = self.eval_expr(expr, program)?;
                self.eval_unary(op, value)
            }

            Expr::Binary { left, op, right } => {
                if *op == BinaryOp::And {
                    let left_value = self.eval_expr(left, program)?;

                    match left_value {
                        Value::Bool(false) => return Ok(Value::Bool(false)),
                        Value::Bool(true) => {
                            let right_value = self.eval_expr(right, program)?;

                            return match right_value {
                                Value::Bool(value) => Ok(Value::Bool(value)),
                                other => Err(format!(
                                    "&& right side must be bool, got {}.",
                                    other.type_name()
                                )),
                            };
                        }
                        other => {
                            return Err(format!(
                                "&& left side must be bool, got {}.",
                                other.type_name()
                            ));
                        }
                    }
                }

                if *op == BinaryOp::Or {
                    let left_value = self.eval_expr(left, program)?;

                    match left_value {
                        Value::Bool(true) => return Ok(Value::Bool(true)),
                        Value::Bool(false) => {
                            let right_value = self.eval_expr(right, program)?;

                            return match right_value {
                                Value::Bool(value) => Ok(Value::Bool(value)),
                                other => Err(format!(
                                    "|| right side must be bool, got {}.",
                                    other.type_name()
                                )),
                            };
                        }
                        other => {
                            return Err(format!(
                                "|| left side must be bool, got {}.",
                                other.type_name()
                            ));
                        }
                    }
                }

                let left_value = self.eval_expr(left, program)?;
                let right_value = self.eval_expr(right, program)?;

                self.eval_binary(left_value, op, right_value)
            }
        }
    }

    fn eval_unary(&self, op: &UnaryOp, value: Value) -> Result<Value, String> {
        match (op, value) {
            (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
            (UnaryOp::Not, other) => Err(format!("! expected bool, got {}.", other.type_name())),

            (UnaryOp::Neg, Value::Int(value)) => Ok(Value::Int(-value)),
            (UnaryOp::Neg, other) => Err(format!("- expected int, got {}.", other.type_name())),
        }
    }

    fn eval_index(&self, target: Value, index: Value) -> Result<Value, String> {
        match (target, index) {
            (Value::Array(values), Value::Int(index)) => {
                if index < 0 {
                    return Err("Array index cannot be negative.".to_string());
                }

                values
                    .get(index as usize)
                    .cloned()
                    .ok_or_else(|| format!("Array index out of bounds: {}", index))
            }

            (Value::Dict(values), Value::Str(key)) => values
                .get(&key)
                .cloned()
                .ok_or_else(|| format!("Dictionary key not found: '{}'.", key)),

            (Value::Array(_), other) => Err(format!(
                "Array index must be int, got {}.",
                other.type_name()
            )),

            (Value::Dict(_), other) => Err(format!(
                "Dictionary index must be str, got {}.",
                other.type_name()
            )),

            (other, _) => Err(format!(
                "Cannot index value of type {}.",
                other.type_name()
            )),
        }
    }

    fn eval_binary(&self, left: Value, op: &BinaryOp, right: Value) -> Result<Value, String> {
        match (left, op, right) {
            (Value::Int(a), BinaryOp::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), BinaryOp::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), BinaryOp::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(_), BinaryOp::Div, Value::Int(0)) => {
                Err("Cannot divide by zero.".to_string())
            }
            (Value::Int(a), BinaryOp::Div, Value::Int(b)) => Ok(Value::Int(a / b)),

            (Value::Str(a), BinaryOp::Add, Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
            (Value::Str(a), BinaryOp::Add, b) => Ok(Value::Str(format!("{}{}", a, b.to_output()))),
            (a, BinaryOp::Add, Value::Str(b)) => Ok(Value::Str(format!("{}{}", a.to_output(), b))),

            (a, BinaryOp::Eq, b) => Ok(Value::Bool(a == b)),
            (a, BinaryOp::NotEq, b) => Ok(Value::Bool(a != b)),

            (Value::Int(a), BinaryOp::Less, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), BinaryOp::Greater, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), BinaryOp::LessEq, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), BinaryOp::GreaterEq, Value::Int(b)) => Ok(Value::Bool(a >= b)),

            (left, op, right) => Err(format!(
                "Invalid binary operation: {} {:?} {}.",
                left.type_name(),
                op,
                right.type_name()
            )),
        }
    }

    fn call_function_by_name(
        &mut self,
        name: &str,
        args: Vec<Value>,
        program: &Program,
    ) -> Result<Value, String> {
        if let Some(value) = self.call_builtin(name, &args)? {
            return Ok(value);
        }

        let index = self
            .functions
            .get(name)
            .copied()
            .ok_or_else(|| format!("Undefined function '{}'.", name))?;

        let function = match &program.items[index] {
            Item::Function(function) => function,
            _ => return Err(format!("Internal error: '{}' is not a function.", name)),
        };

        self.call_user_function(function, args, program)
    }

    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Option<Value>, String> {
        match name {
            "len" => {
                if args.len() != 1 {
                    return Err("len() expects 1 argument.".to_string());
                }

                match &args[0] {
                    Value::Str(value) => Ok(Some(Value::Int(value.chars().count() as i64))),
                    Value::Array(values) => Ok(Some(Value::Int(values.len() as i64))),
                    Value::Dict(values) => Ok(Some(Value::Int(values.len() as i64))),
                    other => Err(format!("len() does not support {}.", other.type_name())),
                }
            }

            "input_str" => {
                if args.len() != 1 {
                    return Err("input_str() expects 1 argument.".to_string());
                }

                let prompt = match &args[0] {
                    Value::Str(value) => value,
                    other => {
                        return Err(format!(
                            "input_str() prompt must be str, got {}.",
                            other.type_name()
                        ));
                    }
                };

                print!("{}", prompt);
                io::stdout().flush().map_err(|error| error.to_string())?;

                let mut input = String::new();

                io::stdin()
                    .read_line(&mut input)
                    .map_err(|error| error.to_string())?;

                Ok(Some(Value::Str(input.trim_end().to_string())))
            }

            "input_int" => {
                if args.len() != 1 {
                    return Err("input_int() expects 1 argument.".to_string());
                }

                let prompt = match &args[0] {
                    Value::Str(value) => value,
                    other => {
                        return Err(format!(
                            "input_int() prompt must be str, got {}.",
                            other.type_name()
                        ));
                    }
                };

                print!("{}", prompt);
                io::stdout().flush().map_err(|error| error.to_string())?;

                let mut input = String::new();

                io::stdin()
                    .read_line(&mut input)
                    .map_err(|error| error.to_string())?;

                let trimmed = input.trim();

                let number = trimmed.parse::<i64>().map_err(|_| {
                    format!("input_int() expected a valid integer, got '{}'.", trimmed)
                })?;

                Ok(Some(Value::Int(number)))
            }

            "lower" => {
                if args.len() != 1 {
                    return Err("lower() expects 1 argument.".to_string());
                }

                match &args[0] {
                    Value::Str(value) => Ok(Some(Value::Str(value.to_lowercase()))),
                    other => Err(format!("lower() expected str, got {}.", other.type_name())),
                }
            }

            "upper" => {
                if args.len() != 1 {
                    return Err("upper() expects 1 argument.".to_string());
                }

                match &args[0] {
                    Value::Str(value) => Ok(Some(Value::Str(value.to_uppercase()))),
                    other => Err(format!("upper() expected str, got {}.", other.type_name())),
                }
            }

            "trim" => {
                if args.len() != 1 {
                    return Err("trim() expects 1 argument.".to_string());
                }

                match &args[0] {
                    Value::Str(value) => Ok(Some(Value::Str(value.trim().to_string()))),
                    other => Err(format!("trim() expected str, got {}.", other.type_name())),
                }
            }

            "contains" => {
                if args.len() != 2 {
                    return Err("contains() expects 2 arguments.".to_string());
                }

                match (&args[0], &args[1]) {
                    (Value::Str(value), Value::Str(needle)) => {
                        Ok(Some(Value::Bool(value.contains(needle))))
                    }

                    (left, right) => Err(format!(
                        "contains() expected str, str, got {}, {}.",
                        left.type_name(),
                        right.type_name()
                    )),
                }
            }

            _ => Ok(None),
        }
    }

    fn call_user_function(
        &mut self,
        function: &Function,
        args: Vec<Value>,
        program: &Program,
    ) -> Result<Value, String> {
        if function.params.len() != args.len() {
            return Err(format!(
                "Function '{}' expected {} arguments, got {}.",
                function.name,
                function.params.len(),
                args.len()
            ));
        }

        self.scopes.push(HashMap::new());

        for (param, value) in function.params.iter().zip(args.into_iter()) {
            if !self.type_matches(&param.ty, &value) {
                self.scopes.pop();

                return Err(format!(
                    "Function '{}' parameter '{}' expected {}, got {}.",
                    function.name,
                    param.name,
                    self.clean_type(&param.ty),
                    value.type_name()
                ));
            }

            self.define_var(param.name.clone(), value);
        }

        let signal = self.execute_block(&function.body, program)?;

        self.scopes.pop();

        match signal {
            ExecSignal::Return(value) => {
                if let Some(return_type) = &function.return_type {
                    if !self.type_matches(return_type, &value) {
                        return Err(format!(
                            "Function '{}' expected return type {}, got {}.",
                            function.name,
                            self.clean_type(return_type),
                            value.type_name()
                        ));
                    }
                }

                Ok(value)
            }

            ExecSignal::None => {
                if let Some(return_type) = &function.return_type {
                    Err(format!(
                        "Function '{}' expected return type {}, but returned nothing.",
                        function.name,
                        self.clean_type(return_type)
                    ))
                } else {
                    Ok(Value::Bool(true))
                }
            }

            ExecSignal::Break => Err("break used outside of loop.".to_string()),

            ExecSignal::Continue => Err("continue used outside of loop.".to_string()),
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

    fn set_var(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }

        Err(format!("Undefined variable '{}'.", name))
    }

    fn type_matches(&self, expected: &str, value: &Value) -> bool {
        let expected = self.clean_type(expected);

        match expected.as_str() {
            "int" => matches!(value, Value::Int(_)),
            "str" => matches!(value, Value::Str(_)),
            "bool" => matches!(value, Value::Bool(_)),
            "arr" => matches!(value, Value::Array(_)),
            "dict" => matches!(value, Value::Dict(_)),
            other if other.starts_with("arr_") => matches!(value, Value::Array(_)),
            other if other.starts_with("dict_") => matches!(value, Value::Dict(_)),
            _ => false,
        }
    }

    fn clean_type(&self, ty: &str) -> String {
        if let Some(value) = ty.strip_prefix("cre_") {
            value.to_string()
        } else {
            ty.to_string()
        }
    }
}