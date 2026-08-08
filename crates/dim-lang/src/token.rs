use logos::Logos;

#[derive(Logos, Debug, PartialEq, Eq, Clone, Copy)]
#[logos(skip r"[ \t\n\r]+")]
pub enum Token {
    #[regex(r"[0-9]+")]
    Int,

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

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,
}

/// Lexed token with source span. `Copy` so peg can parse `&[Spanned]`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Spanned {
    pub token: Token,
    pub start: usize,
    pub end: usize,
}
