mod ast;
mod interpreter;
mod lexer;
mod parser;
mod token;

use std::env;
use std::fs;

use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = if args.len() > 1 {
        &args[1]
    } else {
        "src/main.tsst"
    };

    let source = match fs::read_to_string(file_path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Could not read file '{}': {}", file_path, error);
            return;
        }
    };

    let mut lexer = Lexer::new(&source);

    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(error) => {
            eprintln!("Lexer error: {}", error);
            return;
        }
    };

    let mut parser = Parser::new(tokens);

    let program = match parser.parse_program() {
        Ok(program) => program,
        Err(error) => {
            eprintln!("Parser error: {}", error);
            return;
        }
    };

    let mut interpreter = Interpreter::new();

    if let Err(error) = interpreter.run(&program) {
        eprintln!("Runtime error: {}", error);
    }
}