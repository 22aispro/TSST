use std::collections::HashMap;

use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
enum Type {
    Int,
    Str,
    Bool,
    Arr,
    Dict,
    Void,
    Unknown,
}

#[derive(Debug, Clone)]
struct FunctionSig {
    params: Vec<Type>,
    return_type: Type,
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    functions: HashMap<String, FunctionSig>,
    loop_depth: usize,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            loop_depth: 0,
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), String> {
        self.collect_functions(program)?;

        for item in &program.items {
            if let Item::VarDecl(var_decl) = item {
                self.check_var_decl(var_decl)?;
                return Err(format!(
                    "Top-level variable '{}' is not supported; declare it inside a function",
                    var_decl.name
                ));
            }
        }

        for item in &program.items {
            if let Item::Function(function) = item {
                self.check_function(function)?;
            }
        }

        Ok(())
    }

    fn collect_functions(&mut self, program: &Program) -> Result<(), String> {
        for item in &program.items {
            if let Item::Function(function) = item {
                if self.functions.contains_key(&function.name) {
                    return Err(format!("Function '{}' already exists", function.name));
                }

                let mut params = Vec::new();

                for param in &function.params {
                    params.push(Self::type_from_name(&param.ty)?);
                }

                let return_type = match &function.return_type {
                    Some(return_type) => Self::type_from_name(return_type)?,
                    None => Type::Void,
                };

                self.functions.insert(
                    function.name.clone(),
                    FunctionSig {
                        params,
                        return_type,
                    },
                );
            }
        }

        let main = self
            .functions
            .get("main")
            .ok_or_else(|| "No main function exists".to_string())?;

        if !main.params.is_empty() {
            return Err("main function cannot have parameters".to_string());
        }

        Ok(())
    }

    fn check_function(&mut self, function: &Function) -> Result<(), String> {
        self.push_scope();

        for param in &function.params {
            let ty = Self::type_from_name(&param.ty)?;
            self.define_var(param.name.clone(), ty)?;
        }

        let expected_return = match &function.return_type {
            Some(return_type) => Self::type_from_name(return_type)?,
            None => Type::Void,
        };

        let mut found_return = false;

        for stmt in &function.body {
            self.check_stmt(stmt, &expected_return, &mut found_return)?;
        }

        self.pop_scope();

        if expected_return != Type::Void && !Self::block_definitely_returns(&function.body) {
            return Err(format!(
                "Function '{}' should return {} on every path",
                function.name,
                expected_return.name()
            ));
        }

        Ok(())
    }

    fn block_definitely_returns(body: &[Stmt]) -> bool {
        body.iter().any(Self::stmt_definitely_returns)
    }

    fn stmt_definitely_returns(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Return(_) => true,
            Stmt::If(if_stmt) => {
                Self::block_definitely_returns(&if_stmt.then_body)
                    && if_stmt
                        .else_body
                        .as_deref()
                        .is_some_and(Self::block_definitely_returns)
            }
            _ => false,
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        expected_return: &Type,
        found_return: &mut bool,
    ) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl(var_decl) => self.check_var_decl(var_decl),

            Stmt::MacroCall(call) => self.check_macro_call(call),

            Stmt::FunctionCall(call) => {
                self.check_function_call(&call.name, &call.args, false)?;
                Ok(())
            }

            Stmt::Assignment(assignment) => self.check_assignment(assignment),

            Stmt::If(if_stmt) => {
                let condition = self.check_expr(&if_stmt.condition)?;

                if !Self::types_compatible(&Type::Bool, &condition) {
                    return Err(format!(
                        "If condition must be bool, got {}",
                        condition.name()
                    ));
                }

                self.check_scoped_body(&if_stmt.then_body, expected_return, found_return)?;

                if let Some(else_body) = &if_stmt.else_body {
                    self.check_scoped_body(else_body, expected_return, found_return)?;
                }

                Ok(())
            }

            Stmt::While(while_stmt) => {
                let condition = self.check_expr(&while_stmt.condition)?;

                if !Self::types_compatible(&Type::Bool, &condition) {
                    return Err(format!(
                        "While condition must be bool, got {}",
                        condition.name()
                    ));
                }

                self.loop_depth += 1;

                self.check_scoped_body(&while_stmt.body, expected_return, found_return)?;

                self.loop_depth -= 1;

                Ok(())
            }

            Stmt::For(for_stmt) => {
                self.push_scope();

                self.check_var_decl(&for_stmt.initializer)?;

                let condition = self.check_expr(&for_stmt.condition)?;

                if !Self::types_compatible(&Type::Bool, &condition) {
                    return Err(format!(
                        "For condition must be bool, got {}",
                        condition.name()
                    ));
                }

                self.loop_depth += 1;

                self.check_scoped_body(&for_stmt.body, expected_return, found_return)?;

                self.loop_depth -= 1;

                self.check_assignment(&for_stmt.update)?;

                self.pop_scope();

                Ok(())
            }

            Stmt::ForEach(for_each_stmt) => {
                let iterable = self.check_expr(&for_each_stmt.iterable)?;

                match iterable {
                    Type::Arr | Type::Unknown => {}

                    Type::Dict => {
                        let item_ty = Self::type_from_name(&for_each_stmt.item_ty)?;

                        if item_ty != Type::Str {
                            return Err(format!(
                                "For-each over dict gives string keys, but '{}' is {}",
                                for_each_stmt.item_name,
                                item_ty.name()
                            ));
                        }
                    }

                    other => {
                        return Err(format!(
                            "For-each expected arr or dict, got {}",
                            other.name()
                        ));
                    }
                }

                let item_ty = Self::type_from_name(&for_each_stmt.item_ty)?;

                self.push_scope();

                self.define_var(for_each_stmt.item_name.clone(), item_ty)?;

                self.loop_depth += 1;

                for stmt in &for_each_stmt.body {
                    self.check_stmt(stmt, expected_return, found_return)?;
                }

                self.loop_depth -= 1;

                self.pop_scope();

                Ok(())
            }

            Stmt::Break => {
                if self.loop_depth == 0 {
                    return Err("break used outside of loop".to_string());
                }

                Ok(())
            }

            Stmt::Continue => {
                if self.loop_depth == 0 {
                    return Err("continue used outside of loop".to_string());
                }

                Ok(())
            }

            Stmt::Return(return_stmt) => {
                let actual = self.check_expr(&return_stmt.value)?;

                if expected_return == &Type::Void {
                    return Err(
                        "Cannot return a value from a function with no return type".to_string()
                    );
                }

                if !Self::types_compatible(expected_return, &actual) {
                    return Err(format!(
                        "Return expected {}, got {}",
                        expected_return.name(),
                        actual.name()
                    ));
                }

                *found_return = true;

                Ok(())
            }
        }
    }

    fn check_var_decl(&mut self, var_decl: &VarDecl) -> Result<(), String> {
        let expected = Self::type_from_name(&var_decl.ty)?;
        let actual = self.check_expr(&var_decl.value)?;

        if !Self::types_compatible(&expected, &actual) {
            return Err(format!(
                "Variable '{}' expected {}, got {}",
                var_decl.name,
                expected.name(),
                actual.name()
            ));
        }

        self.define_var(var_decl.name.clone(), expected)?;

        Ok(())
    }

    fn check_scoped_body(
        &mut self,
        body: &[Stmt],
        expected_return: &Type,
        found_return: &mut bool,
    ) -> Result<(), String> {
        self.push_scope();
        let result = body
            .iter()
            .try_for_each(|stmt| self.check_stmt(stmt, expected_return, found_return));
        self.pop_scope();
        result
    }

    fn check_assignment(&mut self, assignment: &Assignment) -> Result<(), String> {
        let expected = self
            .get_var(&assignment.name)
            .ok_or(format!("Unknown variable '{}'", assignment.name))?;

        let actual = self.check_expr(&assignment.value)?;

        if !Self::types_compatible(&expected, &actual) {
            return Err(format!(
                "Cannot assign {} to variable '{}' of type {}",
                actual.name(),
                assignment.name,
                expected.name()
            ));
        }

        Ok(())
    }

    fn check_macro_call(&mut self, call: &MacroCall) -> Result<(), String> {
        match call.name.as_str() {
            "cons" => {
                for arg in &call.args {
                    self.check_expr(arg)?;
                }

                Ok(())
            }

            "push" => {
                if call.args.len() != 2 {
                    return Err(format!("push! expected 2 args, got {}", call.args.len()));
                }

                let array_name = match &call.args[0] {
                    Expr::Ident(name) => name,
                    _ => return Err("push! first argument must be an array variable".to_string()),
                };

                let array_type = self
                    .get_var(array_name)
                    .ok_or(format!("Unknown variable '{array_name}'"))?;

                if !Self::types_compatible(&Type::Arr, &array_type) {
                    return Err(format!("push! expected arr, got {}", array_type.name()));
                }

                self.check_expr(&call.args[1])?;

                Ok(())
            }

            "set" => {
                if call.args.len() != 3 {
                    return Err(format!("set! expected 3 args, got {}", call.args.len()));
                }

                let dict_name = match &call.args[0] {
                    Expr::Ident(name) => name,
                    _ => {
                        return Err("set! first argument must be a dictionary variable".to_string())
                    }
                };

                let dict_type = self
                    .get_var(dict_name)
                    .ok_or(format!("Unknown variable '{dict_name}'"))?;

                if !Self::types_compatible(&Type::Dict, &dict_type) {
                    return Err(format!("set! expected dict, got {}", dict_type.name()));
                }

                let key_type = self.check_expr(&call.args[1])?;

                if !Self::types_compatible(&Type::Str, &key_type) {
                    return Err(format!("set! key must be str, got {}", key_type.name()));
                }

                self.check_expr(&call.args[2])?;

                Ok(())
            }

            other => Err(format!("Unknown macro '{other}!'")),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<Type, String> {
        match expr {
            Expr::Int(_) => Ok(Type::Int),

            Expr::Str(_) => Ok(Type::Str),

            Expr::Bool(_) => Ok(Type::Bool),

            Expr::Ident(name) => self
                .get_var(name)
                .ok_or(format!("Unknown variable '{name}'")),

            Expr::ArrayLiteral(items) => {
                for item in items {
                    self.check_expr(item)?;
                }

                Ok(Type::Arr)
            }

            Expr::DictLiteral(pairs) => {
                for (_, value) in pairs {
                    self.check_expr(value)?;
                }

                Ok(Type::Dict)
            }

            Expr::Index { target, index } => {
                let target_type = self.check_expr(target)?;
                let index_type = self.check_expr(index)?;

                match target_type {
                    Type::Arr => {
                        if !Self::types_compatible(&Type::Int, &index_type) {
                            return Err(format!(
                                "Array index must be int, got {}",
                                index_type.name()
                            ));
                        }

                        Ok(Type::Unknown)
                    }

                    Type::Dict => {
                        if !Self::types_compatible(&Type::Str, &index_type) {
                            return Err(format!(
                                "Dictionary index must be str, got {}",
                                index_type.name()
                            ));
                        }

                        Ok(Type::Unknown)
                    }

                    Type::Unknown => Ok(Type::Unknown),

                    other => Err(format!("Cannot index {}", other.name())),
                }
            }

            Expr::Call { name, args } => self.check_function_call(name, args, true),

            Expr::Unary { op, expr } => {
                let actual = self.check_expr(expr)?;
                let expected = match op {
                    UnaryOp::Not => Type::Bool,
                    UnaryOp::Neg => Type::Int,
                };

                if Self::types_compatible(&expected, &actual) {
                    Ok(expected)
                } else {
                    Err(format!(
                        "Unary operator expected {}, got {}",
                        expected.name(),
                        actual.name()
                    ))
                }
            }

            Expr::Binary { left, op, right } => {
                let left_type = self.check_expr(left)?;
                let right_type = self.check_expr(right)?;

                self.check_binary(&left_type, op, &right_type)
            }
        }
    }

    fn check_function_call(
        &mut self,
        name: &str,
        args: &[Expr],
        used_as_expr: bool,
    ) -> Result<Type, String> {
        if matches!(
            name,
            "gui_button_call" | "gui_profile_dashboard" | "gui_get_string" | "gui_get_int"
        ) {
            if let Some(result) = self.check_builtin_call(name, args)? {
                return Ok(result);
            }
        }

        if name.starts_with("gui_") {
            for arg in args {
                self.check_expr(arg)?;
            }
            return Ok(Type::Unknown);
        }

        if let Some(result) = self.check_builtin_call(name, args)? {
            return Ok(result);
        }

        let sig = self
            .functions
            .get(name)
            .cloned()
            .ok_or(format!("Unknown function '{name}'"))?;

        if sig.params.len() != args.len() {
            return Err(format!(
                "Function '{}' expected {} args, got {}",
                name,
                sig.params.len(),
                args.len()
            ));
        }

        for (index, arg) in args.iter().enumerate() {
            let expected = &sig.params[index];
            let actual = self.check_expr(arg)?;

            if !Self::types_compatible(expected, &actual) {
                return Err(format!(
                    "Function '{}' argument {} expected {}, got {}",
                    name,
                    index + 1,
                    expected.name(),
                    actual.name()
                ));
            }
        }

        if used_as_expr && sig.return_type == Type::Void {
            return Err(format!("Function '{name}' does not return a value"));
        }

        Ok(sig.return_type)
    }

    fn check_binary(&self, left: &Type, op: &BinaryOp, right: &Type) -> Result<Type, String> {
        match op {
            BinaryOp::Add => {
                if left == &Type::Str || right == &Type::Str {
                    return Ok(Type::Str);
                }

                if Self::types_compatible(&Type::Int, left)
                    && Self::types_compatible(&Type::Int, right)
                {
                    return Ok(Type::Int);
                }

                if left == &Type::Unknown || right == &Type::Unknown {
                    return Ok(Type::Unknown);
                }

                Err(format!("Cannot add {} and {}", left.name(), right.name()))
            }

            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                if Self::types_compatible(&Type::Int, left)
                    && Self::types_compatible(&Type::Int, right)
                {
                    return Ok(Type::Int);
                }

                Err(format!(
                    "Operator requires int and int, got {} and {}",
                    left.name(),
                    right.name()
                ))
            }

            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEq | BinaryOp::GreaterEq => {
                if Self::types_compatible(&Type::Int, left)
                    && Self::types_compatible(&Type::Int, right)
                {
                    return Ok(Type::Bool);
                }

                Err(format!(
                    "Comparison requires int and int, got {} and {}",
                    left.name(),
                    right.name()
                ))
            }

            BinaryOp::Eq | BinaryOp::NotEq => Ok(Type::Bool),

            BinaryOp::And | BinaryOp::Or => {
                if Self::types_compatible(&Type::Bool, left)
                    && Self::types_compatible(&Type::Bool, right)
                {
                    Ok(Type::Bool)
                } else {
                    Err(format!(
                        "Boolean operator requires bool and bool, got {} and {}",
                        left.name(),
                        right.name()
                    ))
                }
            }
        }
    }

    fn check_builtin_call(&mut self, name: &str, args: &[Expr]) -> Result<Option<Type>, String> {
        let signature = match name {
            "len" => None,
            "input_str" => Some((vec![Type::Str], Type::Str)),
            "input_int" => Some((vec![Type::Str], Type::Int)),
            "to_int" => Some((vec![Type::Str], Type::Int)),
            "lower" | "upper" | "trim" => Some((vec![Type::Str], Type::Str)),
            "contains" => Some((vec![Type::Str, Type::Str], Type::Bool)),
            "os_run" => Some((vec![Type::Str, Type::Arr], Type::Int)),
            "os_capture" => Some((vec![Type::Str, Type::Arr], Type::Str)),
            "os_get_env" => Some((vec![Type::Str], Type::Str)),
            "os_set_env" => Some((vec![Type::Str, Type::Str], Type::Bool)),
            "os_read_file" => Some((vec![Type::Str], Type::Str)),
            "os_write_file" => Some((vec![Type::Str, Type::Str], Type::Bool)),
            "os_exists" => Some((vec![Type::Str], Type::Bool)),
            "os_sleep" => Some((vec![Type::Int], Type::Bool)),
            "os_current_dir" => Some((vec![], Type::Str)),
            "gui_button_call" => Some((vec![Type::Str, Type::Str], Type::Bool)),
            "gui_profile_dashboard" => Some((
                vec![
                    Type::Arr,
                    Type::Str,
                    Type::Int,
                    Type::Int,
                    Type::Str,
                    Type::Str,
                    Type::Str,
                ],
                Type::Bool,
            )),
            "gui_get_string" => Some((vec![Type::Str], Type::Str)),
            "gui_get_int" => Some((vec![Type::Str], Type::Int)),
            _ => return Ok(None),
        };

        if name == "len" {
            if args.len() != 1 {
                return Err(format!("len expected 1 arg, got {}", args.len()));
            }

            let actual = self.check_expr(&args[0])?;
            return match actual {
                Type::Str | Type::Arr | Type::Dict | Type::Unknown => Ok(Some(Type::Int)),
                other => Err(format!("len cannot be used on {}", other.name())),
            };
        }

        let (params, result) = signature.expect("known builtin has a signature");

        if params.len() != args.len() {
            return Err(format!(
                "{} expected {} args, got {}",
                name,
                params.len(),
                args.len()
            ));
        }

        for (index, (expected, arg)) in params.iter().zip(args).enumerate() {
            let actual = self.check_expr(arg)?;
            if !Self::types_compatible(expected, &actual) {
                return Err(format!(
                    "{} argument {} expected {}, got {}",
                    name,
                    index + 1,
                    expected.name(),
                    actual.name()
                ));
            }
        }

        Ok(Some(result))
    }

    fn type_from_name(name: &str) -> Result<Type, String> {
        let name = name.strip_prefix("cre_").unwrap_or(name);

        match name {
            "int" => Ok(Type::Int),
            "str" => Ok(Type::Str),
            "bool" => Ok(Type::Bool),
            "arr" => Ok(Type::Arr),
            "dict" => Ok(Type::Dict),

            other => Err(format!("Unknown type '{other}'")),
        }
    }

    fn types_compatible(expected: &Type, actual: &Type) -> bool {
        expected == actual || expected == &Type::Unknown || actual == &Type::Unknown
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_var(&mut self, name: String, ty: Type) -> Result<(), String> {
        let scope = self
            .scopes
            .last_mut()
            .ok_or("Internal typechecker error: missing scope".to_string())?;

        if scope.contains_key(&name) {
            return Err(format!("Variable '{name}' already exists in this scope"));
        }

        scope.insert(name, ty);

        Ok(())
    }

    fn get_var(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }

        None
    }
}

impl Type {
    fn name(&self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::Str => "str",
            Type::Bool => "bool",
            Type::Arr => "arr",
            Type::Dict => "dict",
            Type::Void => "void",
            Type::Unknown => "unknown",
        }
    }
}
