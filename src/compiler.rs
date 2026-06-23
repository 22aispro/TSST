use crate::ast::{
    Assignment, BinaryOp, Expr, ForEachStmt, ForStmt, Function, FunctionCall, IfStmt, Item,
    MacroCall, Program, ReturnStmt, Stmt, UnaryOp, VarDecl, WhileStmt,
};
use crate::generated_runtime::runtime_source;

pub struct Compiler {
    temp_counter: usize,
    current_return_type: Option<String>,
    current_function_name: String,
    loop_stack: Vec<Option<Assignment>>,
    uses_gui: bool,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            temp_counter: 0,
            current_return_type: None,
            current_function_name: String::new(),
            loop_stack: Vec::new(),
            uses_gui: false,
        }
    }

    pub fn compile_program(&mut self, program: &Program) -> Result<String, String> {
        let mut functions = String::new();
        self.uses_gui = false;

        for item in &program.items {
            match item {
                Item::Function(function) => {
                    functions.push_str(&self.compile_function(function)?);
                    functions.push('\n');
                }

                Item::VarDecl(_) => {
                    return Err(
                        "Top-level variables are not supported by tsst build yet.".to_string()
                    );
                }
            }
        }

        let mut output = runtime_source(self.uses_gui);
        output.push('\n');
        output.push_str(&functions);

        if !program.items.iter().any(|item| {
            if let Item::Function(function) = item {
                function.name == "main"
            } else {
                false
            }
        }) {
            return Err("Cannot build project because no main function exists.".to_string());
        }

        output.push_str(
            r#"
fn main() {
    match tsst_main() {
        Ok(_) => {}
        Err(error) => {
            eprintln!("Runtime error: {}", error);
            std::process::exit(1);
        }
    }
}
"#,
        );

        Ok(output)
    }

    fn compile_function(&mut self, function: &Function) -> Result<String, String> {
        self.current_return_type = function.return_type.clone();
        self.current_function_name = function.name.clone();

        let mut output = String::new();

        let params = function
            .params
            .iter()
            .map(|param| format!("mut {}: RtValue", rust_var(&param.name)))
            .collect::<Vec<String>>()
            .join(", ");

        output.push_str(&format!(
            "fn {}({}) -> Result<RtValue, String> {{\n",
            rust_fn(&function.name),
            params
        ));

        for param in &function.params {
            output.push_str(&indent_line(
                1,
                &format!(
                    "assert_type(&{}, \"{}\", \"parameter '{}'\")?;",
                    rust_var(&param.name),
                    clean_type(&param.ty),
                    param.name
                ),
            ));
        }

        for stmt in &function.body {
            output.push_str(&self.compile_stmt(stmt, 1)?);
        }

        if let Some(return_type) = &function.return_type {
            output.push_str(&indent_line(
                1,
                &format!(
                    "Err(format!(\"Function '{}' expected return type {}, but returned nothing.\"))",
                    function.name,
                    clean_type(return_type)
                ),
            ));
        } else {
            output.push_str(&indent_line(1, "Ok(RtValue::Bool(true))"));
        }

        output.push_str("}\n");

        self.current_return_type = None;
        self.current_function_name.clear();

        Ok(output)
    }

    fn compile_stmt(&mut self, stmt: &Stmt, indent: usize) -> Result<String, String> {
        match stmt {
            Stmt::VarDecl(var_decl) => self.compile_var_decl(var_decl, indent),
            Stmt::MacroCall(macro_call) => self.compile_macro_call(macro_call, indent),
            Stmt::FunctionCall(function_call) => {
                self.compile_function_call_stmt(function_call, indent)
            }
            Stmt::Assignment(assignment) => self.compile_assignment(assignment, indent),
            Stmt::If(if_stmt) => self.compile_if(if_stmt, indent),
            Stmt::While(while_stmt) => self.compile_while(while_stmt, indent),
            Stmt::For(for_stmt) => self.compile_for(for_stmt, indent),
            Stmt::ForEach(for_each_stmt) => self.compile_for_each(for_each_stmt, indent),
            Stmt::Break => Ok(indent_line(indent, "break;")),
            Stmt::Continue => self.compile_continue(indent),
            Stmt::Return(return_stmt) => self.compile_return(return_stmt, indent),
        }
    }

    fn compile_var_decl(&mut self, var_decl: &VarDecl, indent: usize) -> Result<String, String> {
        let value = self.compile_expr(&var_decl.value)?;

        let mut output = String::new();

        output.push_str(&indent_line(
            indent,
            &format!("let mut {} = {};", rust_var(&var_decl.name), value),
        ));

        output.push_str(&indent_line(
            indent,
            &format!(
                "assert_type(&{}, \"{}\", \"variable '{}'\")?;",
                rust_var(&var_decl.name),
                clean_type(&var_decl.ty),
                var_decl.name
            ),
        ));

        Ok(output)
    }

    fn compile_assignment(
        &mut self,
        assignment: &Assignment,
        indent: usize,
    ) -> Result<String, String> {
        let temp = self.next_temp();
        let value = self.compile_expr(&assignment.value)?;

        let mut output = String::new();

        output.push_str(&indent_line(indent, "{"));
        output.push_str(&indent_line(indent + 1, &format!("let {temp} = {value};")));
        output.push_str(&indent_line(
            indent + 1,
            &format!(
                "ensure_same_type(&{}, &{}, \"{}\")?;",
                rust_var(&assignment.name),
                temp,
                assignment.name
            ),
        ));
        output.push_str(&indent_line(
            indent + 1,
            &format!("{} = {};", rust_var(&assignment.name), temp),
        ));
        output.push_str(&indent_line(indent, "}"));

        Ok(output)
    }

    fn compile_macro_call(
        &mut self,
        macro_call: &MacroCall,
        indent: usize,
    ) -> Result<String, String> {
        match macro_call.name.as_str() {
            "cons" => {
                if macro_call.args.len() != 1 {
                    return Err("cons!() expects 1 argument.".to_string());
                }

                let value = self.compile_expr(&macro_call.args[0])?;

                Ok(indent_line(
                    indent,
                    &format!("println!(\"{}\", {}.to_output());", "{}", value),
                ))
            }

            "push" => {
                if macro_call.args.len() != 2 {
                    return Err("push!() expects 2 arguments.".to_string());
                }

                let target = match &macro_call.args[0] {
                    Expr::Ident(name) => rust_var(name),
                    _ => {
                        return Err("push!() first argument must be an array variable.".to_string())
                    }
                };

                let value = self.compile_expr(&macro_call.args[1])?;

                Ok(indent_line(
                    indent,
                    &format!("push_value(&mut {target}, {value}, \"push!\")?;"),
                ))
            }

            "set" => {
                if macro_call.args.len() != 3 {
                    return Err("set!() expects 3 arguments.".to_string());
                }

                let target = match &macro_call.args[0] {
                    Expr::Ident(name) => rust_var(name),
                    _ => {
                        return Err(
                            "set!() first argument must be a dictionary variable.".to_string()
                        );
                    }
                };

                let key = self.compile_expr(&macro_call.args[1])?;
                let value = self.compile_expr(&macro_call.args[2])?;

                Ok(indent_line(
                    indent,
                    &format!("set_value(&mut {target}, {key}, {value}, \"set!\")?;"),
                ))
            }

            other => Err(format!("Unknown macro '{other}!'.")),
        }
    }

    fn compile_function_call_stmt(
        &mut self,
        function_call: &FunctionCall,
        indent: usize,
    ) -> Result<String, String> {
        let expression = Expr::Call {
            name: function_call.name.clone(),
            args: function_call.args.clone(),
        };
        let compiled = self.compile_expr(&expression)?;
        Ok(indent_line(indent, &format!("{compiled};")))
    }

    fn compile_if(&mut self, if_stmt: &IfStmt, indent: usize) -> Result<String, String> {
        let condition = self.compile_expr(&if_stmt.condition)?;
        let mut output = String::new();

        output.push_str(&indent_line(
            indent,
            &format!("if expect_bool({condition}, \"if condition\")? {{"),
        ));

        for stmt in &if_stmt.then_body {
            output.push_str(&self.compile_stmt(stmt, indent + 1)?);
        }

        output.push_str(&indent_line(indent, "}"));

        if let Some(else_body) = &if_stmt.else_body {
            output.pop();
            output.push_str(" else {\n");

            for stmt in else_body {
                output.push_str(&self.compile_stmt(stmt, indent + 1)?);
            }

            output.push_str(&indent_line(indent, "}"));
        }

        Ok(output)
    }

    fn compile_while(&mut self, while_stmt: &WhileStmt, indent: usize) -> Result<String, String> {
        let condition = self.compile_expr(&while_stmt.condition)?;
        let mut output = String::new();

        output.push_str(&indent_line(
            indent,
            &format!("while expect_bool({condition}, \"while condition\")? {{"),
        ));

        self.loop_stack.push(None);
        let body = while_stmt
            .body
            .iter()
            .map(|stmt| self.compile_stmt(stmt, indent + 1))
            .collect::<Result<String, String>>();
        self.loop_stack.pop();
        output.push_str(&body?);

        output.push_str(&indent_line(indent, "}"));

        Ok(output)
    }

    fn compile_for(&mut self, for_stmt: &ForStmt, indent: usize) -> Result<String, String> {
        let init_value = self.compile_expr(&for_stmt.initializer.value)?;
        let condition = self.compile_expr(&for_stmt.condition)?;
        let mut output = String::new();

        output.push_str(&indent_line(indent, "{"));
        output.push_str(&indent_line(
            indent + 1,
            &format!(
                "let mut {} = {};",
                rust_var(&for_stmt.initializer.name),
                init_value
            ),
        ));
        output.push_str(&indent_line(
            indent + 1,
            &format!(
                "assert_type(&{}, \"{}\", \"for initializer '{}'\")?;",
                rust_var(&for_stmt.initializer.name),
                clean_type(&for_stmt.initializer.ty),
                for_stmt.initializer.name
            ),
        ));

        output.push_str(&indent_line(
            indent + 1,
            &format!("while expect_bool({condition}, \"for condition\")? {{"),
        ));

        self.loop_stack.push(Some(for_stmt.update.clone()));
        let body = for_stmt
            .body
            .iter()
            .map(|stmt| self.compile_stmt(stmt, indent + 2))
            .collect::<Result<String, String>>();
        self.loop_stack.pop();
        output.push_str(&body?);
        output.push_str(&self.compile_assignment(&for_stmt.update, indent + 2)?);

        output.push_str(&indent_line(indent + 1, "}"));
        output.push_str(&indent_line(indent, "}"));

        Ok(output)
    }

    fn compile_for_each(
        &mut self,
        for_each_stmt: &ForEachStmt,
        indent: usize,
    ) -> Result<String, String> {
        let iterable = self.compile_expr(&for_each_stmt.iterable)?;
        let temp = self.next_temp();
        let keys = self.next_temp();
        let mut output = String::new();

        output.push_str(&indent_line(indent, "{"));
        output.push_str(&indent_line(
            indent + 1,
            &format!("let {temp} = {iterable};"),
        ));
        output.push_str(&indent_line(indent + 1, &format!("match {temp} {{")));

        output.push_str(&indent_line(indent + 2, "RtValue::Array(values) => {"));
        output.push_str(&indent_line(indent + 3, "for item in values {"));
        output.push_str(&indent_line(
            indent + 4,
            &format!("let mut {} = item;", rust_var(&for_each_stmt.item_name)),
        ));
        output.push_str(&indent_line(
            indent + 4,
            &format!(
                "assert_type(&{}, \"{}\", \"for-each item '{}'\")?;",
                rust_var(&for_each_stmt.item_name),
                clean_type(&for_each_stmt.item_ty),
                for_each_stmt.item_name
            ),
        ));

        self.loop_stack.push(None);
        let array_body = for_each_stmt
            .body
            .iter()
            .map(|stmt| self.compile_stmt(stmt, indent + 4))
            .collect::<Result<String, String>>();
        self.loop_stack.pop();
        output.push_str(&array_body?);

        output.push_str(&indent_line(indent + 3, "}"));
        output.push_str(&indent_line(indent + 2, "}"));

        output.push_str(&indent_line(indent + 2, "RtValue::Dict(values) => {"));
        output.push_str(&indent_line(
            indent + 3,
            &format!("let mut {keys}: Vec<String> = values.keys().cloned().collect();"),
        ));
        output.push_str(&indent_line(indent + 3, &format!("{keys}.sort();")));
        output.push_str(&indent_line(indent + 3, &format!("for key in {keys} {{")));
        output.push_str(&indent_line(
            indent + 4,
            &format!(
                "let mut {} = RtValue::Str(key);",
                rust_var(&for_each_stmt.item_name)
            ),
        ));
        output.push_str(&indent_line(
            indent + 4,
            &format!(
                "assert_type(&{}, \"{}\", \"for-each key '{}'\")?;",
                rust_var(&for_each_stmt.item_name),
                clean_type(&for_each_stmt.item_ty),
                for_each_stmt.item_name
            ),
        ));

        self.loop_stack.push(None);
        let dict_body = for_each_stmt
            .body
            .iter()
            .map(|stmt| self.compile_stmt(stmt, indent + 4))
            .collect::<Result<String, String>>();
        self.loop_stack.pop();
        output.push_str(&dict_body?);

        output.push_str(&indent_line(indent + 3, "}"));
        output.push_str(&indent_line(indent + 2, "}"));

        output.push_str(&indent_line(
            indent + 2,
            "other => return Err(format!(\"for-each expected arr or dict, got {}.\", other.type_name())),",
        ));

        output.push_str(&indent_line(indent + 1, "}"));
        output.push_str(&indent_line(indent, "}"));

        Ok(output)
    }

    fn compile_return(
        &mut self,
        return_stmt: &ReturnStmt,
        indent: usize,
    ) -> Result<String, String> {
        let value = self.compile_expr(&return_stmt.value)?;
        let temp = self.next_temp();
        let mut output = String::new();

        output.push_str(&indent_line(indent, "{"));
        output.push_str(&indent_line(indent + 1, &format!("let {temp} = {value};")));

        if let Some(return_type) = &self.current_return_type {
            output.push_str(&indent_line(
                indent + 1,
                &format!(
                    "assert_type(&{}, \"{}\", \"return value from '{}'\")?;",
                    temp,
                    clean_type(return_type),
                    self.current_function_name
                ),
            ));
        }

        output.push_str(&indent_line(indent + 1, &format!("return Ok({temp});")));
        output.push_str(&indent_line(indent, "}"));

        Ok(output)
    }

    fn compile_continue(&mut self, indent: usize) -> Result<String, String> {
        let mut output = String::new();

        if let Some(Some(update)) = self.loop_stack.last().cloned() {
            output.push_str(&self.compile_assignment(&update, indent)?);
        }

        output.push_str(&indent_line(indent, "continue;"));
        Ok(output)
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Int(value) => Ok(format!("RtValue::Int({value})")),
            Expr::Str(value) => Ok(format!("RtValue::Str(String::from({value:?}))")),
            Expr::Bool(value) => Ok(format!("RtValue::Bool({value})")),
            Expr::Ident(name) => Ok(format!("{}.clone()", rust_var(name))),

            Expr::ArrayLiteral(values) => {
                let compiled = values
                    .iter()
                    .map(|value| self.compile_expr(value))
                    .collect::<Result<Vec<String>, String>>()?
                    .join(", ");

                Ok(format!("RtValue::Array(vec![{compiled}])"))
            }

            Expr::DictLiteral(values) => {
                if values.is_empty() {
                    return Ok("RtValue::Dict(HashMap::new())".to_string());
                }

                let mut pairs = Vec::new();

                for (key, value) in values {
                    pairs.push(format!(
                        "(String::from({:?}), {})",
                        key,
                        self.compile_expr(value)?
                    ));
                }

                Ok(format!(
                    "RtValue::Dict(HashMap::from([{}]))",
                    pairs.join(", ")
                ))
            }

            Expr::Index { target, index } => {
                let target = self.compile_expr(target)?;
                let index = self.compile_expr(index)?;

                Ok(format!("index_value({target}, {index})?"))
            }

            Expr::Call { name, args } => {
                if name.starts_with("gui_") {
                    self.uses_gui = true;
                }

                let args = self.compile_args(args)?;

                match name.as_str() {
                    "len" => Ok(format!("builtin_len({args})?")),
                    "input_str" => Ok(format!("builtin_input_str({args})?")),
                    "input_int" => Ok(format!("builtin_input_int({args})?")),
                    "lower" => Ok(format!("builtin_lower({args})?")),
                    "upper" => Ok(format!("builtin_upper({args})?")),
                    "trim" => Ok(format!("builtin_trim({args})?")),
                    "contains" => Ok(format!("builtin_contains({args})?")),
                    "os_run" => Ok(format!("builtin_os_run({args})?")),
                    "os_capture" => Ok(format!("builtin_os_capture({args})?")),
                    "os_get_env" => Ok(format!("builtin_os_get_env({args})?")),
                    "os_set_env" => Ok(format!("builtin_os_set_env({args})?")),
                    "os_read_file" => Ok(format!("builtin_os_read_file({args})?")),
                    "os_write_file" => Ok(format!("builtin_os_write_file({args})?")),
                    "os_exists" => Ok(format!("builtin_os_exists({args})?")),
                    "os_sleep" => Ok(format!("builtin_os_sleep({args})?")),
                    "os_current_dir" => Ok("builtin_os_current_dir()?".to_string()),
                    _ => Ok(format!("{}({})?", rust_fn(name), args)),
                }
            }

            Expr::Unary { op, expr } => {
                let value = self.compile_expr(expr)?;

                match op {
                    UnaryOp::Not => Ok(format!("unary_not({value})?")),
                    UnaryOp::Neg => Ok(format!("unary_neg({value})?")),
                }
            }

            Expr::Binary { left, op, right } => {
                if *op == BinaryOp::And {
                    let temp = self.next_temp();
                    let left = self.compile_expr(left)?;
                    let right = self.compile_expr(right)?;

                    return Ok(format!(
                        "{{ let {temp} = expect_bool({left}, \"&& left\")?; if !{temp} {{ RtValue::Bool(false) }} else {{ RtValue::Bool(expect_bool({right}, \"&& right\")?) }} }}"
                    ));
                }

                if *op == BinaryOp::Or {
                    let temp = self.next_temp();
                    let left = self.compile_expr(left)?;
                    let right = self.compile_expr(right)?;

                    return Ok(format!(
                        "{{ let {temp} = expect_bool({left}, \"|| left\")?; if {temp} {{ RtValue::Bool(true) }} else {{ RtValue::Bool(expect_bool({right}, \"|| right\")?) }} }}"
                    ));
                }

                let left = self.compile_expr(left)?;
                let right = self.compile_expr(right)?;

                match op {
                    BinaryOp::Add => Ok(format!("binary_add({left}, {right})?")),
                    BinaryOp::Sub => Ok(format!("binary_sub({left}, {right})?")),
                    BinaryOp::Mul => Ok(format!("binary_mul({left}, {right})?")),
                    BinaryOp::Div => Ok(format!("binary_div({left}, {right})?")),
                    BinaryOp::Eq => Ok(format!("RtValue::Bool({left} == {right})")),
                    BinaryOp::NotEq => Ok(format!("RtValue::Bool({left} != {right})")),
                    BinaryOp::Less => Ok(format!("binary_less({left}, {right})?")),
                    BinaryOp::Greater => Ok(format!("binary_greater({left}, {right})?")),
                    BinaryOp::LessEq => Ok(format!("binary_less_eq({left}, {right})?")),
                    BinaryOp::GreaterEq => Ok(format!("binary_greater_eq({left}, {right})?")),
                    BinaryOp::And | BinaryOp::Or => unreachable!(),
                }
            }
        }
    }

    fn compile_args(&mut self, args: &[Expr]) -> Result<String, String> {
        args.iter()
            .map(|arg| self.compile_expr(arg))
            .collect::<Result<Vec<String>, String>>()
            .map(|values| values.join(", "))
    }

    fn next_temp(&mut self) -> String {
        self.temp_counter += 1;
        format!("__tsst_temp_{}", self.temp_counter)
    }
}

fn rust_fn(name: &str) -> String {
    format!("tsst_{}", sanitize_ident(name))
}

fn rust_var(name: &str) -> String {
    format!("v_{}", sanitize_ident(name))
}

fn sanitize_ident(name: &str) -> String {
    let mut result = String::new();

    for char_value in name.chars() {
        if char_value.is_ascii_alphanumeric() || char_value == '_' {
            result.push(char_value);
        } else {
            result.push('_');
        }
    }

    if result.is_empty() {
        result.push('_');
    }

    if result.chars().next().unwrap().is_ascii_digit() {
        result.insert(0, '_');
    }

    result
}

fn clean_type(ty: &str) -> String {
    if let Some(value) = ty.strip_prefix("cre_") {
        value.to_string()
    } else {
        ty.to_string()
    }
}

fn indent_line(indent: usize, value: &str) -> String {
    format!("{}{}\n", "    ".repeat(indent), value)
}
