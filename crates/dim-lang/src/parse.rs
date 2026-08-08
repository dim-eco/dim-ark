use logos::Logos;

use crate::ast::{c, Expr};
use crate::error::Error;
use crate::token::{Spanned, Token};

pub fn lex(src: &str) -> Result<Vec<Spanned>, Error> {
    let mut tokens = Vec::new();
    let mut lexer = Token::lexer(src);
    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => {
                let span = lexer.span();
                tokens.push(Spanned {
                    token,
                    start: span.start,
                    end: span.end,
                });
            }
            Err(()) => {
                return Err(Error::Lex {
                    span: lexer.span(),
                    slice: lexer.slice().to_owned(),
                });
            }
        }
    }
    Ok(tokens)
}

pub fn parse(src: &str) -> Result<Expr, Error> {
    let tokens = lex(src)?;
    expr_parser::expr(&tokens, src).map_err(|err| Error::Parse {
        location: err.location,
        expected: format!("{:?}", err.expected),
    })
}

peg::parser! {
    grammar expr_parser() for [Spanned] {
        pub rule expr(src: &str) -> Expr = precedence!{
            x:(@) [Spanned { token: Token::Plus, .. }] y:@ {
                c("__intrinsic_add", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Minus, .. }] y:@ {
                c("__intrinsic_sub", vec![x, y])
            }
            --
            x:(@) [Spanned { token: Token::Star, .. }] y:@ {
                c("__intrinsic_mul", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Slash, .. }] y:@ {
                c("__intrinsic_div", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Percent, .. }] y:@ {
                c("__intrinsic_mod", vec![x, y])
            }
            --
            [Spanned { token: Token::Int, start, end }] {
                Expr::Lit(src[start..end].to_owned())
            }
            [Spanned { token: Token::LParen, .. }] e:expr(src) [Spanned { token: Token::RParen, .. }] {
                e
            }
        }
    }
}
