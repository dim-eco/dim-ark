mod ast;
mod error;
mod parse;
mod token;
mod vm;

pub use ast::Expr;
pub use error::Error;
pub use parse::{lex, parse};
pub use token::{Spanned, Token};
pub use vm::eval;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn eval_src(src: &str) -> Result<String, Error> {
    eval(&parse(src)?)
}

#[cfg(test)]
mod tests {
    use super::eval_src;

    fn expect_ok(src: &str, expected: &str) {
        assert_eq!(eval_src(src).unwrap(), expected);
    }

    #[test]
    fn add() {
        expect_ok("1+2", "3");
    }

    #[test]
    fn sub() {
        expect_ok("10-3", "7");
    }

    #[test]
    fn mul() {
        expect_ok("4*5", "20");
    }

    #[test]
    fn div() {
        expect_ok("20/4", "5");
    }

    #[test]
    fn rem() {
        expect_ok("10%3", "1");
    }

    #[test]
    fn precedence() {
        expect_ok("1+2*3", "7");
    }

    #[test]
    fn parens() {
        expect_ok("(1+2)*3", "9");
    }

    #[test]
    fn left_assoc() {
        expect_ok("10-3-2", "5");
    }

    #[test]
    fn nested() {
        expect_ok("(10+2)/(3+1)", "3");
    }

    #[test]
    fn whitespace() {
        expect_ok(" 1 + 2 * 3 ", "7");
    }
}
