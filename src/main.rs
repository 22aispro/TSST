mod ast;
mod interpreter;
mod lexer;
mod parser;
mod token;

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("src/main.tsst")
    };

    let mut imported_files = HashSet::new();

    let source = match read_source_with_imports(&file_path, &mut imported_files) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Import error: {}", error);
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

fn read_source_with_imports(
    file_path: &Path,
    imported_files: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let canonical_path = fs::canonicalize(file_path)
        .map_err(|error| format!("Could not find file '{}': {}", file_path.display(), error))?;

    if imported_files.contains(&canonical_path) {
        return Ok(String::new());
    }

    imported_files.insert(canonical_path.clone());

    let source = fs::read_to_string(&canonical_path)
        .map_err(|error| format!("Could not read file '{}': {}", canonical_path.display(), error))?;

    let base_dir = canonical_path.parent().unwrap_or(Path::new("."));

    let mut combined_source = String::new();

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();

        if trimmed.starts_with("use") {
            let import_path = parse_use_line(trimmed).ok_or(format!(
                "{}:{}: invalid import syntax. Use: use \"file.tsst\";",
                canonical_path.display(),
                line_number
            ))?;

            let full_import_path = base_dir.join(import_path);

            let imported_source = read_source_with_imports(&full_import_path, imported_files)?;

            combined_source.push_str("\n// imported file start\n");
            combined_source.push_str(&imported_source);
            combined_source.push_str("\n// imported file end\n");

            continue;
        }

        combined_source.push_str(line);
        combined_source.push('\n');
    }

    Ok(combined_source)
}

fn parse_use_line(line: &str) -> Option<String> {
    let rest = line.strip_prefix("use")?.trim();

    if !rest.starts_with('"') {
        return None;
    }

    let rest = &rest[1..];

    let end_quote_index = rest.find('"')?;

    let import_path = rest[..end_quote_index].to_string();

    let after_quote = rest[end_quote_index + 1..].trim();

    if after_quote != ";" {
        return None;
    }

    Some(import_path)
}