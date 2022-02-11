/// Concrete Syntax Tree, mirrors the syntax closely with no transformations.
#[derive(Debug)]
pub struct AST {
    pub items: Vec<Item>,
}

pub struct Attr {
    pub name: String,
    pub args: Vec<String>,
}

pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct Item {
    // span: Span,
    pub kind: ItemKind,
}

#[derive(Debug)]
pub enum ItemKind {
    Def(Def),
}

#[derive(Debug)]
pub struct Def {
    pub name: String,
    pub expr: Expr,
}

#[derive(Debug)]
pub struct Expr {
    // span: Span,
    pub kind: ExprKind,
}

#[derive(Debug)]
pub enum ExprKind {
    Literal(Literal),
    Name(String),
    Lambda(String, Box<Expr>),
    Apply(String, Literal),
}

#[derive(Debug)]
pub struct Literal {
    // span: Span,
    pub kind: LitKind,
}

#[derive(Debug)]
pub enum LitKind {
    Int(i64),
}
