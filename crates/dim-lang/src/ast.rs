#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Lit(String),
    Call { name: String, args: Vec<Expr> },
}

pub(crate) fn c(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.to_owned(),
        args,
    }
}
