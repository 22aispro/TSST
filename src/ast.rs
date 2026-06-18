#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    VarDecl(VarDecl),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub is_pub: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub ty: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl(VarDecl),
    MacroCall(MacroCall),
    FunctionCall(FunctionCall),
    Assignment(Assignment),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    ForEach(ForEachStmt),
    Break,
    Continue,
    Return(ReturnStmt),
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub ty: String,
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct MacroCall {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_body: Vec<Stmt>,
    pub else_body: Option<Vec<Stmt>>,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub initializer: VarDecl,
    pub condition: Expr,
    pub update: Assignment,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ForEachStmt {
    pub item_ty: String,
    pub item_name: String,
    pub iterable: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Str(String),
    Bool(bool),
    Ident(String),
    ArrayLiteral(Vec<Expr>),
    DictLiteral(Vec<(String, Expr)>),
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq)]
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

    And,
    Or,
}