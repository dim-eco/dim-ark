#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    ExternType { name: String },
    Data { name: String, ty: Type },
    Assign { name: String, value: Expr },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Named(String),
    Record { fields: Vec<(String, Type)> },
    Map { key: Box<Type>, value: Box<Type> },
    List(Box<Type>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Lit(String),
    Str(String),
    NegInf,
    Name(String),
    Field { base: Box<Expr>, field: String },
    Index { base: Box<Expr>, index: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
    MethodCall {
        base: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    RecordLit(Vec<(String, Expr)>),
    Lambda { params: Vec<String>, body: Box<Expr> },
    Block(Vec<Stmt>),
    Dp(DpBlock),
}

/// `dp { ... }` — structured block with a node list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpBlock {
    pub nodes: Vec<NodeDef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    For {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    If {
        cond: Expr,
        body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    Yield(Expr),
    Expr(Expr),
}

/// One `node { ... }` or `node(name = '...') { ... }`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodeDef {
    pub name: Option<String>,
    pub key: Option<Type>,
    pub payload: Option<Expr>,
    pub next: Option<Expr>,
    pub add: Option<Expr>,
    pub mul: Option<Expr>,
    pub unit: Option<Expr>,
    pub zero: Option<Expr>,
}

pub(crate) fn c(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.to_owned(),
        args,
    }
}

#[cfg(test)]
pub(crate) fn name(s: &str) -> Expr {
    Expr::Name(s.to_owned())
}

#[cfg(test)]
pub(crate) fn field(base: Expr, field: &str) -> Expr {
    Expr::Field {
        base: Box::new(base),
        field: field.to_owned(),
    }
}

#[cfg(test)]
pub(crate) fn index(base: Expr, idx: Expr) -> Expr {
    Expr::Index {
        base: Box::new(base),
        index: Box::new(idx),
    }
}

#[cfg(test)]
pub(crate) fn lambda(params: &[&str], body: Expr) -> Expr {
    Expr::Lambda {
        params: params.iter().map(|p| (*p).to_owned()).collect(),
        body: Box::new(body),
    }
}

#[cfg(test)]
pub(crate) fn lit(s: &str) -> Expr {
    Expr::Lit(s.to_owned())
}

#[cfg(test)]
pub(crate) fn named_ty(s: &str) -> Type {
    Type::Named(s.to_owned())
}
