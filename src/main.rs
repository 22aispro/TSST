mod ast;
mod compiler;
mod generated_runtime;
mod interpreter;
mod lexer;
mod parser;
mod token;
mod typechecker;

use compiler::Compiler;
use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;
use typechecker::TypeChecker;

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        run_file(PathBuf::from("src/main.tsst"));
        return;
    }

    match args[1].as_str() {
        "help" | "--help" | "-h" => {
            print_help();
        }

        "init" => {
            let project_name = args.get(2).map(|value| value.as_str());

            if let Err(error) = init_project(project_name) {
                eprintln!("Init error: {error}");
                std::process::exit(1);
            }
        }

        "install" => {
            if let Err(error) = install_packages() {
                eprintln!("Install error: {error}");
                std::process::exit(1);
            }
        }

        "list" => {
            if let Err(error) = list_packages() {
                eprintln!("List error: {error}");
                std::process::exit(1);
            }
        }

        "remove" => {
            let package_name = match args.get(2) {
                Some(name) => name,
                None => {
                    eprintln!("Remove error: Missing package name.");
                    eprintln!("Usage: tsst remove <package>");
                    std::process::exit(1);
                }
            };

            if let Err(error) = remove_package(package_name) {
                eprintln!("Remove error: {error}");
                std::process::exit(1);
            }
        }

        "update" => {
            if let Err(error) = update_packages() {
                eprintln!("Update error: {error}");
                std::process::exit(1);
            }
        }

        "build" => {
            let file_path = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("main.tsst"));

            if let Err(error) = build_file(file_path) {
                eprintln!("Build error: {error}");
                std::process::exit(1);
            }
        }

        "run" => {
            let file_path = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("main.tsst"));

            run_file(file_path);
        }

        file_path => {
            run_file(PathBuf::from(file_path));
        }
    }
}

fn print_help() {
    println!("TSST");
    println!();
    println!("Usage:");
    println!("  tsst <file.tsst>          Run a TSST file");
    println!("  tsst run <file.tsst>      Run a TSST file");
    println!("  tsst build <file.tsst>    Compile a TSST file into a Rust executable");
    println!("  tsst init [name]          Create a new TSST project");
    println!("  tsst install              Install packages from tsst.json");
    println!("  tsst list                 Show installed packages");
    println!("  tsst remove <package>     Remove an installed package");
    println!("  tsst update               Reinstall all packages from tsst.json");
    println!("  tsst help                 Show this help menu");
    println!();
    println!("Examples:");
    println!("  tsst init MyApp");
    println!("  cd MyApp");
    println!("  tsst install");
    println!("  tsst build main.tsst");
    println!("  tsst run main.tsst");
}

fn run_file(file_path: PathBuf) {
    let mut imported_files = HashSet::new();

    let source = match read_source_with_imports(&file_path, &mut imported_files) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Import error: {error}");
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);

    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(error) => {
            eprintln!("Lexer error: {error}");
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);

    let program = match parser.parse_program() {
        Ok(program) => program,
        Err(error) => {
            eprintln!("Parser error: {error}");
            std::process::exit(1);
        }
    };

    let mut typechecker = TypeChecker::new();

    if let Err(error) = typechecker.check_program(&program) {
        eprintln!("Type error: {error}");
        std::process::exit(1);
    }

    let mut interpreter = Interpreter::new();

    if let Err(error) = interpreter.run(&program) {
        eprintln!("Runtime error: {error}");
        std::process::exit(1);
    }
}

fn build_file(file_path: PathBuf) -> Result<(), String> {
    let mut imported_files = HashSet::new();

    let source = read_source_with_imports(&file_path, &mut imported_files)?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer
        .tokenize()
        .map_err(|error| format!("Lexer error: {error}"))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|error| format!("Parser error: {error}"))?;

    let mut typechecker = TypeChecker::new();
    typechecker
        .check_program(&program)
        .map_err(|error| format!("Type error: {error}"))?;

    let mut compiler = Compiler::new();
    let rust_source = compiler.compile_program(&program)?;

    let project_root = find_project_root(&env::current_dir().map_err(|error| error.to_string())?)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let build_root = project_root.join("build");
    let rust_project = build_root.join("rust");
    let rust_src = rust_project.join("src");
    let release_dir = build_root.join("release");

    fs::create_dir_all(&rust_src).map_err(|error| error.to_string())?;
    fs::create_dir_all(&release_dir).map_err(|error| error.to_string())?;

    let package_name = file_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(sanitize_package_name)
        .unwrap_or_else(|| "tsst_app".to_string());

    let gui_dependencies = if rust_source.contains("enum GuiElement") {
        "\neframe = \"0.29\"\negui = \"0.29\"\n"
    } else {
        ""
    };

    fs::write(
        rust_project.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2021"

[dependencies]{gui_dependencies}
"#
        ),
    )
    .map_err(|error| error.to_string())?;

    fs::write(rust_src.join("main.rs"), rust_source).map_err(|error| error.to_string())?;

    println!("Generated Rust project:");
    println!("  {}", rust_project.display());
    println!();
    println!("Building release executable...");

    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&rust_project)
        .status()
        .map_err(|error| {
            format!(
                "Failed to run cargo. Make sure Rust is installed and cargo is in PATH. {error}"
            )
        })?;

    if !status.success() {
        return Err("Rust build failed.".to_string());
    }

    let exe_name = if cfg!(windows) {
        format!("{package_name}.exe")
    } else {
        package_name.clone()
    };

    let built_exe = rust_project.join("target").join("release").join(&exe_name);
    let final_exe = release_dir.join(&exe_name);

    if built_exe.exists() {
        fs::copy(&built_exe, &final_exe).map_err(|error| error.to_string())?;

        println!();
        println!("Built executable:");
        println!("  {}", final_exe.display());
    } else {
        println!();
        println!("Cargo finished, but executable was not found at:");
        println!("  {}", built_exe.display());
    }

    Ok(())
}

fn init_project(project_name: Option<&str>) -> Result<(), String> {
    let project_dir = match project_name {
        Some(name) => PathBuf::from(name),
        None => env::current_dir().map_err(|error| error.to_string())?,
    };

    fs::create_dir_all(&project_dir).map_err(|error| error.to_string())?;

    let packages_dir = project_dir.join("packages");
    fs::create_dir_all(&packages_dir).map_err(|error| error.to_string())?;

    let main_file = project_dir.join("main.tsst");
    let tsst_json = project_dir.join("tsst.json");

    if !main_file.exists() {
        fs::write(
            &main_file,
            r#"pub fcn main () {
    cons!("Hello from TSST!");
}
"#,
        )
        .map_err(|error| error.to_string())?;
    }

    if !tsst_json.exists() {
        let name = project_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("tsst-app");

        fs::write(
            &tsst_json,
            format!(
                r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "dependencies": {{}}
}}
"#
            ),
        )
        .map_err(|error| error.to_string())?;
    }

    println!("Created TSST project at {}", project_dir.display());
    println!();
    println!("Next:");
    println!("  cd {}", project_dir.display());
    println!("  tsst main.tsst");

    Ok(())
}

fn sanitize_package_name(name: &str) -> String {
    let mut result: String = name
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || value == '_' {
                value
            } else {
                '_'
            }
        })
        .collect();

    if result.is_empty() {
        return "tsst_app".to_string();
    }

    if !result
        .chars()
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic() || value == '_')
    {
        result.insert(0, '_');
    }

    result
}

fn install_packages() -> Result<(), String> {
    let project_root = find_project_root(&env::current_dir().map_err(|error| error.to_string())?)
        .ok_or_else(|| {
        "Could not find tsst.json in this folder or any parent folder.".to_string()
    })?;

    let manifest_path = project_root.join("tsst.json");
    let manifest = fs::read_to_string(&manifest_path).map_err(|error| error.to_string())?;

    let dependencies = parse_dependencies(&manifest)?;

    if dependencies.is_empty() {
        println!("No dependencies found in tsst.json.");
        return Ok(());
    }

    let packages_dir = project_root.join("packages");
    fs::create_dir_all(&packages_dir).map_err(|error| error.to_string())?;

    println!("Installing TSST packages...");
    println!();

    for dependency in dependencies {
        let package_dir = packages_dir.join(&dependency.name);

        if package_dir.exists() {
            fs::remove_dir_all(&package_dir).map_err(|error| error.to_string())?;
        }

        if dependency.source.starts_with("github:") {
            install_github_package(&dependency, &package_dir)?;
        } else if dependency.source.starts_with("path:") {
            install_path_package(&dependency, &package_dir, &project_root)?;
        } else if dependency.source.starts_with("https://")
            || dependency.source.starts_with("http://")
        {
            install_git_url_package(&dependency, &package_dir)?;
        } else {
            return Err(format!(
                "Unsupported dependency source for '{}': {}",
                dependency.name, dependency.source
            ));
        }
    }

    println!();
    println!("Done. Packages installed into:");
    println!("  {}", packages_dir.display());

    Ok(())
}

fn list_packages() -> Result<(), String> {
    let project_root = find_project_root(&env::current_dir().map_err(|error| error.to_string())?)
        .ok_or_else(|| {
        "Could not find tsst.json in this folder or any parent folder.".to_string()
    })?;

    let packages_dir = project_root.join("packages");

    if !packages_dir.exists() {
        println!("No packages folder found.");
        return Ok(());
    }

    let mut packages = Vec::new();

    for entry in fs::read_dir(&packages_dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                packages.push(name.to_string());
            }
        }
    }

    packages.sort();

    if packages.is_empty() {
        println!("No packages installed.");
        return Ok(());
    }

    println!("Installed packages:");

    for package in packages {
        println!("  {package}");
    }

    Ok(())
}

fn remove_package(package_name: &str) -> Result<(), String> {
    let project_root = find_project_root(&env::current_dir().map_err(|error| error.to_string())?)
        .ok_or_else(|| {
        "Could not find tsst.json in this folder or any parent folder.".to_string()
    })?;

    let package_dir = project_root.join("packages").join(package_name);

    if !package_dir.exists() {
        return Err(format!("Package '{package_name}' is not installed."));
    }

    if !package_dir.is_dir() {
        return Err(format!(
            "Package path exists but is not a folder: {}",
            package_dir.display()
        ));
    }

    fs::remove_dir_all(&package_dir).map_err(|error| error.to_string())?;

    println!("Removed package '{package_name}'.");

    Ok(())
}

fn update_packages() -> Result<(), String> {
    let project_root = find_project_root(&env::current_dir().map_err(|error| error.to_string())?)
        .ok_or_else(|| {
        "Could not find tsst.json in this folder or any parent folder.".to_string()
    })?;

    let packages_dir = project_root.join("packages");

    if packages_dir.exists() {
        fs::remove_dir_all(&packages_dir).map_err(|error| error.to_string())?;
    }

    fs::create_dir_all(&packages_dir).map_err(|error| error.to_string())?;

    println!("Updating packages...");
    println!();

    install_packages()
}

fn install_github_package(dependency: &Dependency, package_dir: &Path) -> Result<(), String> {
    let repo = dependency.source.trim_start_matches("github:");
    let url = format!("https://github.com/{repo}.git");

    println!("Installing {} from {}", dependency.name, url);

    let status = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(&url)
        .arg(package_dir)
        .status()
        .map_err(|error| {
            format!("Failed to run git. Make sure Git is installed and available in PATH. {error}")
        })?;

    if !status.success() {
        return Err(format!("Failed to clone package '{}'.", dependency.name));
    }

    Ok(())
}

fn install_git_url_package(dependency: &Dependency, package_dir: &Path) -> Result<(), String> {
    println!("Installing {} from {}", dependency.name, dependency.source);

    let status = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(&dependency.source)
        .arg(package_dir)
        .status()
        .map_err(|error| {
            format!("Failed to run git. Make sure Git is installed and available in PATH. {error}")
        })?;

    if !status.success() {
        return Err(format!("Failed to clone package '{}'.", dependency.name));
    }

    Ok(())
}

fn install_path_package(
    dependency: &Dependency,
    package_dir: &Path,
    project_root: &Path,
) -> Result<(), String> {
    let relative_path = dependency.source.trim_start_matches("path:");
    let source_dir = project_root.join(relative_path);

    if !source_dir.exists() {
        return Err(format!(
            "Local package path does not exist for '{}': {}",
            dependency.name,
            source_dir.display()
        ));
    }

    println!(
        "Installing {} from {}",
        dependency.name,
        source_dir.display()
    );

    copy_dir_all(&source_dir, package_dir)?;

    Ok(())
}

fn copy_dir_all(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|error| error.to_string())?;

    for entry in fs::read_dir(from).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let entry_path = entry.path();
        let target_path = to.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Dependency {
    name: String,
    source: String,
}

fn parse_dependencies(manifest: &str) -> Result<Vec<Dependency>, String> {
    let dependencies_key = "\"dependencies\"";

    let key_index = match manifest.find(dependencies_key) {
        Some(index) => index,
        None => return Ok(Vec::new()),
    };

    let after_key = &manifest[key_index + dependencies_key.len()..];

    let object_start_relative = after_key
        .find('{')
        .ok_or_else(|| "Expected '{' after dependencies in tsst.json.".to_string())?;

    let object_start = key_index + dependencies_key.len() + object_start_relative;

    let object_end = find_matching_brace(manifest, object_start)
        .ok_or_else(|| "Could not find end of dependencies object in tsst.json.".to_string())?;

    let object = &manifest[object_start + 1..object_end];

    let mut dependencies = Vec::new();
    let mut index = 0;

    while index < object.len() {
        let name_start = match find_next_quote(object, index) {
            Some(value) => value,
            None => break,
        };

        let name_end = find_string_end(object, name_start + 1)
            .ok_or_else(|| "Invalid dependency name string in tsst.json.".to_string())?;

        let name = object[name_start + 1..name_end].to_string();

        let colon_index = object[name_end + 1..]
            .find(':')
            .map(|value| value + name_end + 1)
            .ok_or_else(|| format!("Expected ':' after dependency '{name}'."))?;

        let source_start = find_next_quote(object, colon_index + 1)
            .ok_or_else(|| format!("Expected source string for dependency '{name}'."))?;

        let source_end = find_string_end(object, source_start + 1)
            .ok_or_else(|| format!("Invalid source string for dependency '{name}'."))?;

        let source = object[source_start + 1..source_end].to_string();

        dependencies.push(Dependency { name, source });

        index = source_end + 1;
    }

    Ok(dependencies)
}

fn find_matching_brace(value: &str, start: usize) -> Option<usize> {
    let bytes = value.as_bytes();

    if bytes.get(start)? != &b'{' {
        return None;
    }

    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;

    for index in start..value.len() {
        let char_value = value.as_bytes()[index] as char;

        if in_string {
            if escaped {
                escaped = false;
            } else if char_value == '\\' {
                escaped = true;
            } else if char_value == '"' {
                in_string = false;
            }

            continue;
        }

        if char_value == '"' {
            in_string = true;
            continue;
        }

        if char_value == '{' {
            depth += 1;
        } else if char_value == '}' {
            depth -= 1;

            if depth == 0 {
                return Some(index);
            }
        }
    }

    None
}

fn find_next_quote(value: &str, start: usize) -> Option<usize> {
    value[start..].find('"').map(|index| index + start)
}

fn find_string_end(value: &str, start: usize) -> Option<usize> {
    let mut escaped = false;

    for index in start..value.len() {
        let char_value = value.as_bytes()[index] as char;

        if escaped {
            escaped = false;
            continue;
        }

        if char_value == '\\' {
            escaped = true;
            continue;
        }

        if char_value == '"' {
            return Some(index);
        }
    }

    None
}

fn read_source_with_imports(
    file_path: &Path,
    imported_files: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let full_path = normalize_file_path(file_path)?;

    if imported_files.contains(&full_path) {
        return Ok(String::new());
    }

    imported_files.insert(full_path.clone());

    let source = fs::read_to_string(&full_path)
        .map_err(|error| format!("Could not read '{}': {}", full_path.display(), error))?;

    let mut final_source = String::new();

    for line in source.lines() {
        if let Some(import_path) = parse_use_import(line) {
            let resolved_path = resolve_import_path(&full_path, &import_path)?;

            let imported_source = read_source_with_imports(&resolved_path, imported_files)?;

            final_source.push_str(&imported_source);
            final_source.push('\n');
        } else {
            final_source.push_str(line);
            final_source.push('\n');
        }
    }

    Ok(final_source)
}

fn parse_use_import(line: &str) -> Option<String> {
    let trimmed = line.trim();

    if !trimmed.starts_with("use ") {
        return None;
    }

    let first_quote = trimmed.find('"')?;
    let rest = &trimmed[first_quote + 1..];
    let second_quote = rest.find('"')?;

    Some(rest[..second_quote].to_string())
}

fn resolve_import_path(importing_file: &Path, import_path: &str) -> Result<PathBuf, String> {
    let importing_dir = importing_file.parent().ok_or_else(|| {
        format!(
            "Could not get parent folder for '{}'.",
            importing_file.display()
        )
    })?;

    if import_path.contains(':') && !import_path.contains("://") {
        return resolve_package_colon_import(importing_file, import_path);
    }

    let relative_path = importing_dir.join(import_path);

    if relative_path.exists() {
        return normalize_file_path(&relative_path);
    }

    if looks_like_package_slash_import(import_path) {
        return resolve_package_slash_import(importing_file, import_path);
    }

    normalize_file_path(&relative_path)
}

fn resolve_package_colon_import(
    importing_file: &Path,
    import_path: &str,
) -> Result<PathBuf, String> {
    let mut parts = import_path.splitn(2, ':');

    let package_name = parts
        .next()
        .ok_or_else(|| format!("Invalid package import '{import_path}'."))?;

    let module_name = parts
        .next()
        .ok_or_else(|| format!("Invalid package import '{import_path}'."))?;

    if package_name.trim().is_empty() || module_name.trim().is_empty() {
        return Err(format!("Invalid package import '{import_path}'."));
    }

    let project_root = find_project_root_from_file(importing_file)
        .ok_or_else(|| "Could not find tsst.json for package import.".to_string())?;

    let module_path = module_name.replace('.', "/");
    let module_path = ensure_tsst_extension(&module_path);

    normalize_file_path(
        &project_root
            .join("packages")
            .join(package_name)
            .join(module_path),
    )
}

fn resolve_package_slash_import(
    importing_file: &Path,
    import_path: &str,
) -> Result<PathBuf, String> {
    let project_root = find_project_root_from_file(importing_file)
        .ok_or_else(|| "Could not find tsst.json for package import.".to_string())?;

    let module_path = ensure_tsst_extension(import_path);

    normalize_file_path(&project_root.join("packages").join(module_path))
}

fn looks_like_package_slash_import(import_path: &str) -> bool {
    !import_path.starts_with("./")
        && !import_path.starts_with("../")
        && !import_path.ends_with(".tsst")
        && import_path.contains('/')
}

fn ensure_tsst_extension(path: &str) -> String {
    if path.ends_with(".tsst") {
        path.to_string()
    } else {
        format!("{path}.tsst")
    }
}

fn normalize_file_path(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        fs::canonicalize(path)
            .map_err(|error| format!("Could not normalize path '{}': {}", path.display(), error))
    } else {
        Err(format!("File does not exist: {}", path.display()))
    }
}

fn find_project_root_from_file(file_path: &Path) -> Option<PathBuf> {
    let parent = file_path.parent()?;
    find_project_root(parent)
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        if current.join("tsst.json").exists() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> ast::Program {
        let tokens = Lexer::new(source).tokenize().expect("source should lex");
        Parser::new(tokens)
            .parse_program()
            .expect("source should parse")
    }

    #[test]
    fn hello_world_typechecks_and_runs() {
        let program = parse(r#"pub fcn main () { cons!("Hello"); }"#);
        TypeChecker::new()
            .check_program(&program)
            .expect("program should typecheck");
        Interpreter::new()
            .run(&program)
            .expect("program should run");
    }

    #[test]
    fn typechecker_rejects_errors_in_unreachable_branches() {
        let program = parse(
            r#"pub fcn main () {
                if false { cre_int value = "wrong"; }
            }"#,
        );
        let error = TypeChecker::new()
            .check_program(&program)
            .expect_err("type mismatch should be caught before execution");
        assert!(error.contains("expected int, got str"));
    }

    #[test]
    fn block_variables_do_not_escape_their_scope() {
        let program = parse(
            r#"pub fcn main () {
                if true { cre_int hidden = 7; }
                cons!(hidden);
            }"#,
        );
        let error = TypeChecker::new()
            .check_program(&program)
            .expect_err("block variable should be out of scope");
        assert!(error.contains("Unknown variable 'hidden'"));
    }

    #[test]
    fn missing_and_duplicate_main_functions_are_rejected() {
        let missing = parse("fcn helper () {}");
        assert!(TypeChecker::new().check_program(&missing).is_err());

        let duplicate = parse("pub fcn main () {} pub fcn main () {}");
        assert!(TypeChecker::new().check_program(&duplicate).is_err());
    }

    #[test]
    fn typed_functions_must_return_on_every_branch() {
        let program = parse(
            r#"fcn maybe (cre_bool condition) -> int {
                if condition { return 1; }
            }
            pub fcn main () {}"#,
        );
        let error = TypeChecker::new()
            .check_program(&program)
            .expect_err("conditional return is not guaranteed");
        assert!(error.contains("on every path"));
    }

    #[test]
    fn non_gui_programs_generate_a_dependency_free_runtime() {
        let program = parse(r#"pub fcn main () { cons!("small"); }"#);
        let generated = Compiler::new()
            .compile_program(&program)
            .expect("program should compile");
        assert!(!generated.contains("enum GuiElement"));
        assert!(!generated.contains("eframe::"));
    }

    #[test]
    fn classic_for_continue_updates_before_continuing() {
        let program = parse(
            r#"pub fcn main () {
                for (cre_int i = 0; i < 3; i = i + 1) {
                    if i == 1 { continue; }
                }
            }"#,
        );
        let generated = Compiler::new()
            .compile_program(&program)
            .expect("program should compile");
        let continue_position = generated
            .find("continue;")
            .expect("continue should compile");
        let before_continue = &generated[..continue_position];
        assert!(before_continue.rfind("ensure_same_type").is_some());
    }

    #[test]
    fn os_builtins_typecheck_run_and_compile() {
        let program = parse(
            r#"pub fcn main () {
                cre_str directory = os_current_dir();
                cre_bool exists = os_exists(directory);
                cons!(exists);
            }"#,
        );
        TypeChecker::new()
            .check_program(&program)
            .expect("OS calls should typecheck");
        Interpreter::new()
            .run(&program)
            .expect("OS calls should run");

        let generated = Compiler::new()
            .compile_program(&program)
            .expect("OS calls should compile");
        assert!(generated.contains("builtin_os_current_dir()?"));
        assert!(generated.contains("builtin_os_exists"));
    }
}
