#[derive(Debug)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug)]
pub enum Item {
    VarDecl(VarDecl),
    Function(Function),
}

#[derive(Debug)]
pub enum Stmt {
    VarDecl(VarDecl),
    MacroCall(MacroCall),
    Assignment(Assignment),
    If(IfStmt),
}

#[derive(Debug)]
pub struct VarDecl {
    pub ty: String,
    pub name: String,
    pub value: Expr,
}

#[derive(Debug)]
pub struct Assignment {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug)]
pub struct Function {
    pub public: bool,
    pub name: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct MacroCall {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_body: Vec<Stmt>,
    pub else_body: Option<Vec<Stmt>>,
}

#[derive(Debug)]
pub enum Expr {
    Int(i64),
    Str(String),
    Bool(bool),
    Ident(String),

    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
}