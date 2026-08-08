use logos::Logos;

#[derive(Logos, Debug, PartialEq, Eq, Clone, Copy)]
#[logos(skip r"[ \t\n\r]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    #[regex(r"[0-9]+")]
    Int,

    #[regex(r"'([^'\\]|\\.)*'")]
    String,

    #[token("extern")]
    Extern,

    #[token("type")]
    Type,

    #[token("data")]
    Data,

    #[token("dp")]
    Dp,

    #[token("node")]
    Node,

    #[token("for")]
    For,

    #[token("in")]
    In,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("yield")]
    Yield,

    #[token("inf")]
    Inf,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[token("<=")]
    Le,

    #[token(">=")]
    Ge,

    #[token("==")]
    EqEq,

    #[token("!=")]
    Ne,

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token(":")]
    Colon,

    #[token("=")]
    Eq,

    #[token(",")]
    Comma,

    #[token("|")]
    Pipe,

    #[token(".")]
    Dot,
}

/// Lexed token with source span. `Copy` so peg can parse `&[Spanned]`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Spanned {
    pub token: Token,
    pub start: usize,
    pub end: usize,
}
