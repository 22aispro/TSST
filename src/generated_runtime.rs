pub fn runtime_source(include_gui: bool) -> String {
    let source = r#"
#![allow(dead_code, unused_mut)]
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq)]
enum RtValue {
    Int(i64),
    Str(String),
    Bool(bool),
    Array(Vec<RtValue>),
    Dict(HashMap<String, RtValue>),
}

impl RtValue {
    fn type_name(&self) -> &'static str {
        match self {
            RtValue::Int(_) => "int",
            RtValue::Str(_) => "str",
            RtValue::Bool(_) => "bool",
            RtValue::Array(_) => "arr",
            RtValue::Dict(_) => "dict",
        }
    }

    fn to_output(&self) -> String {
        match self {
            RtValue::Int(value) => value.to_string(),
            RtValue::Str(value) => value.clone(),
            RtValue::Bool(value) => value.to_string(),
            RtValue::Array(values) => {
                let parts: Vec<String> = values.iter().map(|value| value.to_output()).collect();
                format!("[{}]", parts.join(", "))
            }
            RtValue::Dict(values) => {
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

fn type_matches(expected: &str, value: &RtValue) -> bool {
    match expected {
        "int" => matches!(value, RtValue::Int(_)),
        "str" => matches!(value, RtValue::Str(_)),
        "bool" => matches!(value, RtValue::Bool(_)),
        "arr" => matches!(value, RtValue::Array(_)),
        "dict" => matches!(value, RtValue::Dict(_)),
        other if other.starts_with("arr_") => matches!(value, RtValue::Array(_)),
        other if other.starts_with("dict_") => matches!(value, RtValue::Dict(_)),
        _ => false,
    }
}

fn assert_type(value: &RtValue, expected: &str, context: &str) -> Result<(), String> {
    if type_matches(expected, value) {
        Ok(())
    } else {
        Err(format!(
            "Type mismatch for {}. Expected {}, got {}.",
            context,
            expected,
            value.type_name()
        ))
    }
}

fn ensure_same_type(old_value: &RtValue, new_value: &RtValue, name: &str) -> Result<(), String> {
    if old_value.type_name() == new_value.type_name() {
        Ok(())
    } else {
        Err(format!(
            "Type mismatch assigning '{}'. Expected {}, got {}.",
            name,
            old_value.type_name(),
            new_value.type_name()
        ))
    }
}

fn expect_bool(value: RtValue, context: &str) -> Result<bool, String> {
    match value {
        RtValue::Bool(value) => Ok(value),
        other => Err(format!("{} must be bool, got {}.", context, other.type_name())),
    }
}

fn expect_str(value: RtValue, context: &str) -> Result<String, String> {
    match value {
        RtValue::Str(value) => Ok(value),
        other => Err(format!("{} must be str, got {}.", context, other.type_name())),
    }
}

fn expect_int(value: RtValue, context: &str) -> Result<i64, String> {
    match value {
        RtValue::Int(value) => Ok(value),
        other => Err(format!("{} must be int, got {}.", context, other.type_name())),
    }
}

fn unary_not(value: RtValue) -> Result<RtValue, String> {
    Ok(RtValue::Bool(!expect_bool(value, "!")?))
}

fn unary_neg(value: RtValue) -> Result<RtValue, String> {
    Ok(RtValue::Int(-expect_int(value, "-")?))
}

fn binary_add(left: RtValue, right: RtValue) -> Result<RtValue, String> {
    match (left, right) {
        (RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a + b)),
        (RtValue::Str(a), RtValue::Str(b)) => Ok(RtValue::Str(format!("{}{}", a, b))),
        (RtValue::Str(a), b) => Ok(RtValue::Str(format!("{}{}", a, b.to_output()))),
        (a, RtValue::Str(b)) => Ok(RtValue::Str(format!("{}{}", a.to_output(), b))),
        (left, right) => Err(format!(
            "Invalid binary operation: {} + {}.",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn binary_sub(left: RtValue, right: RtValue) -> Result<RtValue, String> {
    match (left, right) {
        (RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a - b)),
        (left, right) => Err(format!(
            "Invalid binary operation: {} - {}.",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn binary_mul(left: RtValue, right: RtValue) -> Result<RtValue, String> {
    match (left, right) {
        (RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a * b)),
        (left, right) => Err(format!(
            "Invalid binary operation: {} * {}.",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn binary_div(left: RtValue, right: RtValue) -> Result<RtValue, String> {
    match (left, right) {
        (RtValue::Int(_), RtValue::Int(0)) => Err("Cannot divide by zero.".to_string()),
        (RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a / b)),
        (left, right) => Err(format!(
            "Invalid binary operation: {} / {}.",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn binary_less(left: RtValue, right: RtValue) -> Result<RtValue, String> {
    match (left, right) {
        (RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Bool(a < b)),
        (left, right) => Err(format!(
            "Invalid binary operation: {} < {}.",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn binary_greater(left: RtValue, right: RtValue) -> Result<RtValue, String> {
    match (left, right) {
        (RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Bool(a > b)),
        (left, right) => Err(format!(
            "Invalid binary operation: {} > {}.",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn binary_less_eq(left: RtValue, right: RtValue) -> Result<RtValue, String> {
    match (left, right) {
        (RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Bool(a <= b)),
        (left, right) => Err(format!(
            "Invalid binary operation: {} <= {}.",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn binary_greater_eq(left: RtValue, right: RtValue) -> Result<RtValue, String> {
    match (left, right) {
        (RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Bool(a >= b)),
        (left, right) => Err(format!(
            "Invalid binary operation: {} >= {}.",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn index_value(target: RtValue, index: RtValue) -> Result<RtValue, String> {
    match (target, index) {
        (RtValue::Array(values), RtValue::Int(index)) => {
            if index < 0 {
                return Err("Array index cannot be negative.".to_string());
            }

            values
                .get(index as usize)
                .cloned()
                .ok_or_else(|| format!("Array index out of bounds: {}", index))
        }

        (RtValue::Dict(values), RtValue::Str(key)) => values
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("Dictionary key not found: '{}'.", key)),

        (RtValue::Array(_), other) => Err(format!(
            "Array index must be int, got {}.",
            other.type_name()
        )),

        (RtValue::Dict(_), other) => Err(format!(
            "Dictionary index must be str, got {}.",
            other.type_name()
        )),

        (other, _) => Err(format!(
            "Cannot index value of type {}.",
            other.type_name()
        )),
    }
}

fn push_value(target: &mut RtValue, value: RtValue, context: &str) -> Result<(), String> {
    match target {
        RtValue::Array(values) => {
            values.push(value);
            Ok(())
        }
        other => Err(format!("{} expected arr, got {}.", context, other.type_name())),
    }
}

fn set_value(target: &mut RtValue, key: RtValue, value: RtValue, context: &str) -> Result<(), String> {
    let key = expect_str(key, context)?;

    match target {
        RtValue::Dict(values) => {
            values.insert(key, value);
            Ok(())
        }
        other => Err(format!("{} expected dict, got {}.", context, other.type_name())),
    }
}

fn builtin_len(value: RtValue) -> Result<RtValue, String> {
    match value {
        RtValue::Str(value) => Ok(RtValue::Int(value.chars().count() as i64)),
        RtValue::Array(values) => Ok(RtValue::Int(values.len() as i64)),
        RtValue::Dict(values) => Ok(RtValue::Int(values.len() as i64)),
        other => Err(format!("len() does not support {}.", other.type_name())),
    }
}

fn builtin_input_str(prompt: RtValue) -> Result<RtValue, String> {
    let prompt = expect_str(prompt, "input_str prompt")?;

    print!("{}", prompt);
    io::stdout().flush().map_err(|error| error.to_string())?;

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .map_err(|error| error.to_string())?;

    Ok(RtValue::Str(input.trim_end().to_string()))
}

fn builtin_input_int(prompt: RtValue) -> Result<RtValue, String> {
    let prompt = expect_str(prompt, "input_int prompt")?;

    print!("{}", prompt);
    io::stdout().flush().map_err(|error| error.to_string())?;

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .map_err(|error| error.to_string())?;

    let trimmed = input.trim();

    let number = trimmed
        .parse::<i64>()
        .map_err(|_| format!("input_int() expected a valid integer, got '{}'.", trimmed))?;

    Ok(RtValue::Int(number))
}

fn builtin_lower(value: RtValue) -> Result<RtValue, String> {
    Ok(RtValue::Str(expect_str(value, "lower")?.to_lowercase()))
}

fn builtin_upper(value: RtValue) -> Result<RtValue, String> {
    Ok(RtValue::Str(expect_str(value, "upper")?.to_uppercase()))
}

fn builtin_trim(value: RtValue) -> Result<RtValue, String> {
    Ok(RtValue::Str(expect_str(value, "trim")?.trim().to_string()))
}

fn builtin_contains(value: RtValue, needle: RtValue) -> Result<RtValue, String> {
    let value = expect_str(value, "contains value")?;
    let needle = expect_str(needle, "contains needle")?;

    Ok(RtValue::Bool(value.contains(&needle)))
}

fn os_command_args(program: RtValue, args: RtValue, context: &str) -> Result<(String, Vec<String>), String> {
    let program = expect_str(program, &format!("{} program", context))?;
    let args = match args {
        RtValue::Array(values) => values.into_iter().map(|value| value.to_output()).collect(),
        other => return Err(format!("{} arguments must be arr, got {}.", context, other.type_name())),
    };
    Ok((program, args))
}

fn builtin_os_run(program: RtValue, args: RtValue) -> Result<RtValue, String> {
    let (program, args) = os_command_args(program, args, "os_run")?;
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("os_run() failed to start process: {}", error))?;
    Ok(RtValue::Int(status.code().unwrap_or(-1) as i64))
}

fn builtin_os_capture(program: RtValue, args: RtValue) -> Result<RtValue, String> {
    let (program, args) = os_command_args(program, args, "os_capture")?;
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("os_capture() failed to start process: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "os_capture() process exited with code {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    Ok(RtValue::Str(String::from_utf8_lossy(&output.stdout).to_string()))
}

fn builtin_os_get_env(name: RtValue) -> Result<RtValue, String> {
    let name = expect_str(name, "os_get_env name")?;
    Ok(RtValue::Str(std::env::var(name).unwrap_or_default()))
}

fn builtin_os_set_env(name: RtValue, value: RtValue) -> Result<RtValue, String> {
    let name = expect_str(name, "os_set_env name")?;
    let value = expect_str(value, "os_set_env value")?;
    if name.contains('\0') || value.contains('\0') {
        return Err("os_set_env() values cannot contain NUL bytes.".to_string());
    }
    std::env::set_var(name, value);
    Ok(RtValue::Bool(true))
}

fn builtin_os_read_file(path: RtValue) -> Result<RtValue, String> {
    let path = expect_str(path, "os_read_file path")?;
    std::fs::read_to_string(&path)
        .map(RtValue::Str)
        .map_err(|error| format!("os_read_file() failed for '{}': {}", path, error))
}

fn builtin_os_write_file(path: RtValue, content: RtValue) -> Result<RtValue, String> {
    let path = expect_str(path, "os_write_file path")?;
    let content = expect_str(content, "os_write_file content")?;
    std::fs::write(&path, content)
        .map(|_| RtValue::Bool(true))
        .map_err(|error| format!("os_write_file() failed for '{}': {}", path, error))
}

fn builtin_os_exists(path: RtValue) -> Result<RtValue, String> {
    let path = expect_str(path, "os_exists path")?;
    Ok(RtValue::Bool(std::path::Path::new(&path).exists()))
}

fn builtin_os_sleep(milliseconds: RtValue) -> Result<RtValue, String> {
    let milliseconds = expect_int(milliseconds, "os_sleep milliseconds")?;
    if milliseconds < 0 {
        return Err("os_sleep() duration cannot be negative.".to_string());
    }
    std::thread::sleep(std::time::Duration::from_millis(milliseconds as u64));
    Ok(RtValue::Bool(true))
}

fn builtin_os_current_dir() -> Result<RtValue, String> {
    let path = std::env::current_dir().map_err(|error| format!("os_current_dir() failed: {}", error))?;
    Ok(RtValue::Str(path.to_string_lossy().to_string()))
}

#[derive(Debug, Clone)]
enum GuiElement {
    Title(String),
    Text(String),
    Button(String),
    CallbackButton(String, String),
    Space,
    Stat(String, String),
    MathButton(String, String, String, i64),
    TransferButton(String, String, i64, String, i64),
}

#[derive(Debug, Clone)]
struct GuiStyle {
    mode: String,
    bg: (u8, u8, u8),
    panel: (u8, u8, u8),
    card: (u8, u8, u8),
    button: (u8, u8, u8),
    button_alt: (u8, u8, u8),
    accent: (u8, u8, u8),
    text: (u8, u8, u8),
    muted: (u8, u8, u8),
    border: (u8, u8, u8),
    content_width: f32,
    title_size: f32,
    text_size: f32,
    button_height: f32,
    spacing: f32,
}

impl GuiStyle {
    fn new() -> Self {
        Self {
            mode: "dark".to_string(),
            bg: (12, 13, 18),
            panel: (22, 24, 34),
            card: (9, 10, 15),
            button: (54, 63, 108),
            button_alt: (38, 105, 80),
            accent: (145, 225, 255),
            text: (238, 242, 255),
            muted: (180, 188, 215),
            border: (82, 90, 132),
            content_width: 420.0,
            title_size: 25.0,
            text_size: 15.0,
            button_height: 42.0,
            spacing: 10.0,
        }
    }
}

#[derive(Debug, Clone)]
struct ProfileDashboard {
    operators: Vec<String>,
    selected_var: String,
    horizontal_var: String,
    vertical_var: String,
    hotkey: String,
    change_callback: String,
    save_callback: String,
    search: String,
    active_tab: usize,
}

#[derive(Debug, Clone)]
struct GuiState {
    title: String,
    width: i64,
    height: i64,
    elements: Vec<GuiElement>,
    vars: HashMap<String, i64>,
    strings: HashMap<String, String>,
    status: String,
    style: GuiStyle,
    dashboard: Option<ProfileDashboard>,
}

impl GuiState {
    fn new() -> Self {
        Self {
            title: "TSST App".to_string(),
            width: 600,
            height: 400,
            elements: Vec::new(),
            vars: HashMap::new(),
            strings: HashMap::new(),
            status: "Ready.".to_string(),
            style: GuiStyle::new(),
            dashboard: None,
        }
    }
}

static GUI_STATE: OnceLock<Mutex<GuiState>> = OnceLock::new();

fn gui_state() -> &'static Mutex<GuiState> {
    GUI_STATE.get_or_init(|| Mutex::new(GuiState::new()))
}

fn clamp_color(value: i64) -> u8 {
    value.clamp(0, 255) as u8
}

fn rgb(value: (u8, u8, u8)) -> egui::Color32 {
    egui::Color32::from_rgb(value.0, value.1, value.2)
}

fn read_rgb(r: RtValue, g: RtValue, b: RtValue, context: &str) -> Result<(u8, u8, u8), String> {
    let r = expect_int(r, &format!("{} r", context))?;
    let g = expect_int(g, &format!("{} g", context))?;
    let b = expect_int(b, &format!("{} b", context))?;

    Ok((clamp_color(r), clamp_color(g), clamp_color(b)))
}

fn tsst_gui_window(title: RtValue, width: RtValue, height: RtValue) -> Result<RtValue, String> {
    let title = expect_str(title, "gui_window title")?;
    let width = expect_int(width, "gui_window width")?;
    let height = expect_int(height, "gui_window height")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.title = title;
    state.width = width;
    state.height = height;
    state.elements.clear();
    state.vars.clear();
    state.strings.clear();
    state.status = "Ready.".to_string();
    state.style = GuiStyle::new();
    state.dashboard = None;

    Ok(RtValue::Bool(true))
}

fn tsst_gui_title(text: RtValue) -> Result<RtValue, String> {
    let text = expect_str(text, "gui_title text")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.elements.push(GuiElement::Title(text));

    Ok(RtValue::Bool(true))
}

fn tsst_gui_text(text: RtValue) -> Result<RtValue, String> {
    let text = expect_str(text, "gui_text text")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.elements.push(GuiElement::Text(text));

    Ok(RtValue::Bool(true))
}

fn tsst_gui_button(text: RtValue) -> Result<RtValue, String> {
    let text = expect_str(text, "gui_button text")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.elements.push(GuiElement::Button(text));

    Ok(RtValue::Bool(true))
}

fn tsst_gui_button_call(label: RtValue, callback: RtValue) -> Result<RtValue, String> {
    let label = expect_str(label, "gui_button_call label")?;
    let callback = expect_str(callback, "gui_button_call callback")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.elements.push(GuiElement::CallbackButton(label, callback));

    Ok(RtValue::Bool(true))
}

fn tsst_gui_profile_dashboard(
    operators: RtValue,
    selected: RtValue,
    horizontal: RtValue,
    vertical: RtValue,
    hotkey: RtValue,
    change_callback: RtValue,
    save_callback: RtValue,
) -> Result<RtValue, String> {
    let operators = match operators {
        RtValue::Array(values) => values
            .into_iter()
            .map(|value| expect_str(value, "gui_profile_dashboard operator"))
            .collect::<Result<Vec<_>, _>>()?,
        other => return Err(format!("gui_profile_dashboard operators must be arr, got {}.", other.type_name())),
    };
    let selected = expect_str(selected, "gui_profile_dashboard selected")?;
    let horizontal = expect_int(horizontal, "gui_profile_dashboard horizontal")?;
    let vertical = expect_int(vertical, "gui_profile_dashboard vertical")?;
    let hotkey = expect_str(hotkey, "gui_profile_dashboard hotkey")?;
    let change_callback = expect_str(change_callback, "gui_profile_dashboard change callback")?;
    let save_callback = expect_str(save_callback, "gui_profile_dashboard save callback")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.strings.insert("selected_operator".to_string(), selected);
    state.vars.insert("horizontal".to_string(), horizontal);
    state.vars.insert("vertical".to_string(), vertical);
    state.dashboard = Some(ProfileDashboard {
        operators,
        selected_var: "selected_operator".to_string(),
        horizontal_var: "horizontal".to_string(),
        vertical_var: "vertical".to_string(),
        hotkey,
        change_callback,
        save_callback,
        search: String::new(),
        active_tab: 0,
    });
    Ok(RtValue::Bool(true))
}

fn tsst_gui_get_string(name: RtValue) -> Result<RtValue, String> {
    let name = expect_str(name, "gui_get_string name")?;
    let state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    Ok(RtValue::Str(state.strings.get(&name).cloned().unwrap_or_default()))
}

fn tsst_gui_get_int(name: RtValue) -> Result<RtValue, String> {
    let name = expect_str(name, "gui_get_int name")?;
    let state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    Ok(RtValue::Int(state.vars.get(&name).cloned().unwrap_or(0)))
}

fn tsst_gui_space() -> Result<RtValue, String> {
    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.elements.push(GuiElement::Space);

    Ok(RtValue::Bool(true))
}

fn tsst_gui_var(name: RtValue, value: RtValue) -> Result<RtValue, String> {
    let name = expect_str(name, "gui_var name")?;
    let value = expect_int(value, "gui_var value")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.vars.insert(name, value);

    Ok(RtValue::Bool(true))
}

fn tsst_gui_stat(label: RtValue, var_name: RtValue) -> Result<RtValue, String> {
    let label = expect_str(label, "gui_stat label")?;
    let var_name = expect_str(var_name, "gui_stat var_name")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.elements.push(GuiElement::Stat(label, var_name));

    Ok(RtValue::Bool(true))
}

fn tsst_gui_button_math(
    label: RtValue,
    var_name: RtValue,
    op: RtValue,
    amount: RtValue,
) -> Result<RtValue, String> {
    let label = expect_str(label, "gui_button_math label")?;
    let var_name = expect_str(var_name, "gui_button_math var_name")?;
    let op = expect_str(op, "gui_button_math op")?;
    let amount = expect_int(amount, "gui_button_math amount")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.elements.push(GuiElement::MathButton(label, var_name, op, amount));

    Ok(RtValue::Bool(true))
}

fn tsst_gui_button_transfer(
    label: RtValue,
    from_var: RtValue,
    from_amount: RtValue,
    to_var: RtValue,
    to_amount: RtValue,
) -> Result<RtValue, String> {
    let label = expect_str(label, "gui_button_transfer label")?;
    let from_var = expect_str(from_var, "gui_button_transfer from_var")?;
    let from_amount = expect_int(from_amount, "gui_button_transfer from_amount")?;
    let to_var = expect_str(to_var, "gui_button_transfer to_var")?;
    let to_amount = expect_int(to_amount, "gui_button_transfer to_amount")?;

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state
        .elements
        .push(GuiElement::TransferButton(label, from_var, from_amount, to_var, to_amount));

    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_mode(mode: RtValue) -> Result<RtValue, String> {
    let mode = expect_str(mode, "gui_theme_mode mode")?;

    if mode != "light" && mode != "dark" {
        return Err("gui_theme_mode must be \"light\" or \"dark\".".to_string());
    }

    let mut state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?;

    state.style.mode = mode;

    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_bg(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_bg")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.bg = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_panel(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_panel")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.panel = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_card(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_card")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.card = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_button(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_button")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.button = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_button_alt(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_button_alt")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.button_alt = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_accent(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_accent")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.accent = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_text(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_text")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.text = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_muted(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_muted")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.muted = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_theme_border(r: RtValue, g: RtValue, b: RtValue) -> Result<RtValue, String> {
    let value = read_rgb(r, g, b, "gui_theme_border")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.border = value;
    Ok(RtValue::Bool(true))
}

fn tsst_gui_style_width(width: RtValue) -> Result<RtValue, String> {
    let width = expect_int(width, "gui_style_width width")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.content_width = (width as f32).clamp(260.0, 900.0);
    Ok(RtValue::Bool(true))
}

fn tsst_gui_style_title_size(size: RtValue) -> Result<RtValue, String> {
    let size = expect_int(size, "gui_style_title_size size")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.title_size = (size as f32).clamp(12.0, 80.0);
    Ok(RtValue::Bool(true))
}

fn tsst_gui_style_text_size(size: RtValue) -> Result<RtValue, String> {
    let size = expect_int(size, "gui_style_text_size size")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.text_size = (size as f32).clamp(10.0, 60.0);
    Ok(RtValue::Bool(true))
}

fn tsst_gui_style_button_height(height: RtValue) -> Result<RtValue, String> {
    let height = expect_int(height, "gui_style_button_height height")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.button_height = (height as f32).clamp(24.0, 110.0);
    Ok(RtValue::Bool(true))
}

fn tsst_gui_style_spacing(spacing: RtValue) -> Result<RtValue, String> {
    let spacing = expect_int(spacing, "gui_style_spacing spacing")?;
    let mut state = gui_state().lock().map_err(|_| "Could not lock GUI state.".to_string())?;
    state.style.spacing = (spacing as f32).clamp(0.0, 80.0);
    Ok(RtValue::Bool(true))
}

struct TsstGuiApp {
    state: GuiState,
}

impl eframe::App for TsstGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.style.mode == "light" {
            ctx.set_visuals(egui::Visuals::light());
        } else {
            ctx.set_visuals(egui::Visuals::dark());
        }

        if self.state.dashboard.is_some() {
            if let Some(callback) = render_profile_dashboard(ctx, &mut self.state) {
                run_dashboard_callback(&mut self.state, &callback);
            }
            return;
        }

        let style = self.state.style.clone();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(rgb(style.bg)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(24.0);

                        let panel_width = (ui.available_width() - 56.0)
                            .clamp(320.0, style.content_width + 70.0);

                        ui.vertical_centered(|ui| {
                            egui::Frame::none()
                                .fill(rgb(style.panel))
                                .stroke(egui::Stroke::new(1.0, rgb(style.border)))
                                .inner_margin(egui::Margin::same(22.0))
                                .show(ui, |ui| {
                                    ui.set_width(panel_width);
                                    render_gui(ui, &mut self.state);
                                });
                        });

                        ui.add_space(24.0);
                    });
            });
    }
}

fn run_dashboard_callback(state: &mut GuiState, callback: &str) {
    let sync = gui_state()
        .lock()
        .map(|mut global| *global = state.clone())
        .map_err(|_| "Could not lock GUI state.".to_string());
    let result = sync.and_then(|_| tsst_gui_dispatch_callback(callback));
    if let Ok(global) = gui_state().lock() {
        *state = global.clone();
    }
    state.status = match result {
        Ok(()) => "Saved.".to_string(),
        Err(error) => format!("Callback error: {}", error),
    };
}

fn render_profile_dashboard(ctx: &egui::Context, state: &mut GuiState) -> Option<String> {
    let mut dashboard = state.dashboard.clone()?;
    let style = state.style.clone();
    let mut callback = None;
    let mut selected = state.strings.get(&dashboard.selected_var).cloned().unwrap_or_default();
    let mut horizontal = state.vars.get(&dashboard.horizontal_var).cloned().unwrap_or(0);
    let mut vertical = state.vars.get(&dashboard.vertical_var).cloned().unwrap_or(0);

    egui::TopBottomPanel::top("tsst_dashboard_top")
        .frame(egui::Frame::none().fill(rgb(style.panel)).inner_margin(egui::Margin::same(10.0)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(dashboard.active_tab == 0, "Combat").clicked() { dashboard.active_tab = 0; }
                if ui.selectable_label(dashboard.active_tab == 1, "Settings").clicked() { dashboard.active_tab = 1; }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save").clicked() { callback = Some(dashboard.save_callback.clone()); }
                    let active = state.vars.get("enabled").cloned().unwrap_or(0) != 0;
                    ui.label(if active { "● ACTIVE" } else { "○ IDLE" });
                });
            });
        });

    egui::SidePanel::right("tsst_dashboard_operators")
        .resizable(false)
        .exact_width(270.0)
        .frame(egui::Frame::none().fill(rgb(style.panel)).stroke(egui::Stroke::new(1.0, rgb(style.border))).inner_margin(egui::Margin::same(12.0)))
        .show(ctx, |ui| {
            ui.set_width(ui.available_width());
            ui.small("OPERATORS");
            ui.add(egui::TextEdit::singleline(&mut dashboard.search).hint_text("Search operators...").desired_width(f32::INFINITY));
            ui.small("ATTACKERS / DEFENDERS");
            let query = dashboard.search.to_ascii_lowercase();
            egui::ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                for operator in &dashboard.operators {
                    if !query.is_empty() && !operator.to_ascii_lowercase().contains(&query) { continue; }
                    if ui.selectable_label(selected == *operator, operator).clicked() {
                        selected = operator.clone();
                        callback = Some(dashboard.change_callback.clone());
                    }
                }
            });
        });

    egui::CentralPanel::default().frame(egui::Frame::none().fill(rgb(style.bg)).inner_margin(egui::Margin::same(12.0))).show(ctx, |ui| {
        egui::Frame::none().fill(rgb(style.panel)).stroke(egui::Stroke::new(1.0, rgb(style.border))).inner_margin(egui::Margin::same(15.0)).show(ui, |ui| {
            ui.horizontal(|ui| { ui.heading("Compensation"); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label("○ IDLE"); }); });
            ui.horizontal(|ui| { ui.label("Hotkey"); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.monospace(&dashboard.hotkey); }); });
        });
        ui.add_space(10.0);
        egui::Frame::none().fill(rgb(style.panel)).stroke(egui::Stroke::new(1.0, rgb(style.border))).inner_margin(egui::Margin::same(15.0)).show(ui, |ui| {
            ui.horizontal(|ui| { ui.heading("Profile"); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.colored_label(rgb(style.accent), &selected); }); });
            ui.horizontal(|ui| { ui.label("Horizontal"); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.colored_label(rgb(style.accent), format!("{:.3}", horizontal as f64 / 1000.0)); }); });
            if ui.add(egui::Slider::new(&mut horizontal, -3000..=3000).show_value(false)).changed() { callback = Some(dashboard.change_callback.clone()); }
            ui.horizontal(|ui| { ui.label("Vertical"); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.colored_label(rgb(style.accent), format!("{:.3}", vertical as f64 / 1000.0)); }); });
            if ui.add(egui::Slider::new(&mut vertical, 0..=15000).show_value(false)).changed() { callback = Some(dashboard.change_callback.clone()); }
        });
    });

    state.strings.insert(dashboard.selected_var.clone(), selected);
    state.vars.insert(dashboard.horizontal_var.clone(), horizontal);
    state.vars.insert(dashboard.vertical_var.clone(), vertical);
    state.dashboard = Some(dashboard);
    callback
}

fn render_gui(ui: &mut egui::Ui, state: &mut GuiState) {
    let elements = state.elements.clone();

    for element in elements {
        match element {
            GuiElement::Title(text) => render_title(ui, state, &text),
            GuiElement::Text(text) => render_text(ui, state, &text),
            GuiElement::Button(text) => render_plain_button(ui, state, &text),
            GuiElement::CallbackButton(label, callback) => {
                render_callback_button(ui, state, &label, &callback)
            }
            GuiElement::Space => ui.add_space(state.style.spacing + 8.0),
            GuiElement::Stat(label, var_name) => render_stat(ui, state, &label, &var_name),
            GuiElement::MathButton(label, var_name, op, amount) => {
                render_math_button(ui, state, &label, &var_name, &op, amount)
            }
            GuiElement::TransferButton(label, from_var, from_amount, to_var, to_amount) => {
                render_transfer_button(
                    ui,
                    state,
                    &label,
                    &from_var,
                    from_amount,
                    &to_var,
                    to_amount,
                )
            }
        }
    }

    render_status(ui, state);
}

fn render_title(ui: &mut egui::Ui, state: &GuiState, text: &str) {
    let style = &state.style;

    ui.vertical_centered(|ui| {
        ui.add_sized(
            [style.content_width, style.title_size + 12.0],
            egui::Label::new(
                egui::RichText::new(text)
                    .size(style.title_size)
                    .strong()
                    .color(rgb(style.text)),
            ),
        );
    });

    ui.add_space(style.spacing);
}

fn render_text(ui: &mut egui::Ui, state: &GuiState, text: &str) {
    let style = &state.style;

    ui.vertical_centered(|ui| {
        ui.add_sized(
            [style.content_width, style.text_size + 10.0],
            egui::Label::new(
                egui::RichText::new(text)
                    .size(style.text_size)
                    .color(rgb(style.muted)),
            ),
        );
    });

    ui.add_space(style.spacing * 0.6);
}

fn render_plain_button(ui: &mut egui::Ui, state: &GuiState, text: &str) {
    let style = &state.style;

    ui.vertical_centered(|ui| {
        let button = egui::Button::new(
            egui::RichText::new(text)
                .size(style.text_size + 1.0)
                .strong()
                .color(rgb(style.text)),
        )
        .fill(rgb(style.button))
        .stroke(egui::Stroke::new(1.0, rgb(style.border)));

        let _ = ui.add_sized([style.content_width, style.button_height], button);
    });

    ui.add_space(style.spacing * 0.8);
}

fn render_callback_button(
    ui: &mut egui::Ui,
    state: &mut GuiState,
    label: &str,
    callback: &str,
) {
    let style = state.style.clone();

    ui.vertical_centered(|ui| {
        let button = egui::Button::new(
            egui::RichText::new(label)
                .size(style.text_size + 1.0)
                .strong()
                .color(rgb(style.text)),
        )
        .fill(rgb(style.button))
        .stroke(egui::Stroke::new(1.0, rgb(style.border)));

        if ui.add_sized([style.content_width, style.button_height], button).clicked() {
            let sync_result = gui_state()
                .lock()
                .map(|mut global| *global = state.clone())
                .map_err(|_| "Could not lock GUI state.".to_string());

            let callback_result = sync_result.and_then(|_| tsst_gui_dispatch_callback(callback));

            if let Ok(global) = gui_state().lock() {
                *state = global.clone();
            }

            state.status = match callback_result {
                Ok(()) => format!("Ran {}.", callback),
                Err(error) => format!("Callback error: {}", error),
            };
        }
    });

    ui.add_space(style.spacing * 0.8);
}

fn render_stat(ui: &mut egui::Ui, state: &GuiState, label: &str, var_name: &str) {
    let value = state.vars.get(var_name).cloned().unwrap_or(0);
    let style = &state.style;

    ui.vertical_centered(|ui| {
        egui::Frame::none()
            .fill(rgb(style.card))
            .stroke(egui::Stroke::new(1.0, rgb(style.border)))
            .inner_margin(egui::Margin::same(14.0))
            .show(ui, |ui| {
                ui.set_width(style.content_width);

                ui.horizontal(|ui| {
                    ui.add_sized(
                        [style.content_width * 0.42, 30.0],
                        egui::Label::new(
                            egui::RichText::new(label)
                                .size(style.text_size)
                                .color(rgb(style.muted)),
                        ),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_sized(
                            [style.content_width * 0.42, 30.0],
                            egui::Label::new(
                                egui::RichText::new(format!("{}", value))
                                    .size(style.title_size)
                                    .strong()
                                    .color(rgb(style.accent)),
                            ),
                        );
                    });
                });
            });
    });

    ui.add_space(style.spacing);
}

fn render_math_button(
    ui: &mut egui::Ui,
    state: &mut GuiState,
    label: &str,
    var_name: &str,
    op: &str,
    amount: i64,
) {
    let style = state.style.clone();

    ui.vertical_centered(|ui| {
        let button = egui::Button::new(
            egui::RichText::new(label)
                .size(style.text_size + 1.0)
                .strong()
                .color(rgb(style.text)),
        )
        .fill(rgb(style.button))
        .stroke(egui::Stroke::new(1.0, rgb(style.border)));

        if ui.add_sized([style.content_width, style.button_height], button).clicked() {
            let current = state.vars.get(var_name).cloned().unwrap_or(0);

            let next = match op {
                "+" => current + amount,
                "-" => current - amount,
                "*" => current * amount,
                "/" => {
                    if amount == 0 {
                        state.status = "Cannot divide by zero.".to_string();
                        current
                    } else {
                        current / amount
                    }
                }
                "=" | "set" => amount,
                _ => {
                    state.status = format!("Unknown operation '{}'.", op);
                    current
                }
            };

            state.vars.insert(var_name.to_string(), next);
            state.status = format!("{} is now {}.", var_name, next);
        }
    });

    ui.add_space(style.spacing * 0.8);
}

fn render_transfer_button(
    ui: &mut egui::Ui,
    state: &mut GuiState,
    label: &str,
    from_var: &str,
    from_amount: i64,
    to_var: &str,
    to_amount: i64,
) {
    let style = state.style.clone();

    ui.vertical_centered(|ui| {
        let button = egui::Button::new(
            egui::RichText::new(label)
                .size(style.text_size + 1.0)
                .strong()
                .color(rgb(style.text)),
        )
        .fill(rgb(style.button_alt))
        .stroke(egui::Stroke::new(1.0, rgb(style.border)));

        if ui.add_sized([style.content_width, style.button_height], button).clicked() {
            let from_current = state.vars.get(from_var).cloned().unwrap_or(0);
            let to_current = state.vars.get(to_var).cloned().unwrap_or(0);

            if from_current < from_amount {
                state.status = format!("Not enough {}.", from_var);
                return;
            }

            state.vars.insert(from_var.to_string(), from_current - from_amount);
            state.vars.insert(to_var.to_string(), to_current + to_amount);

            state.status = format!(
                "{}: -{} | {}: +{}",
                from_var,
                from_amount,
                to_var,
                to_amount
            );
        }
    });

    ui.add_space(style.spacing * 0.8);
}

fn render_status(ui: &mut egui::Ui, state: &GuiState) {
    let style = &state.style;

    ui.add_space(style.spacing);

    ui.vertical_centered(|ui| {
        ui.add_sized(
            [style.content_width, style.text_size + 12.0],
            egui::Label::new(
                egui::RichText::new(&state.status)
                    .size(style.text_size)
                    .color(rgb(style.muted)),
            ),
        );
    });
}

fn tsst_gui_show() -> Result<RtValue, String> {
    let state = gui_state()
        .lock()
        .map_err(|_| "Could not lock GUI state.".to_string())?
        .clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([state.width as f32, state.height as f32]),
        ..Default::default()
    };

    eframe::run_native(
        &state.title.clone(),
        options,
        Box::new(move |_cc| {
            Ok(Box::new(TsstGuiApp {
                state: state.clone(),
            }))
        }),
    )
    .map_err(|error| error.to_string())?;

    Ok(RtValue::Bool(true))
}
"#;

    if include_gui {
        source.to_string()
    } else {
        source
            .split_once("\n#[derive(Debug, Clone)]\nenum GuiElement")
            .map(|(base, _)| base.replace("use std::sync::{Mutex, OnceLock};\n", ""))
            .unwrap_or_else(|| source.to_string())
    }
}
