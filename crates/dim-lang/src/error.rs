use std::fmt;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Lex { span: Range<usize>, slice: String },
    Parse { location: usize, expected: String },
    Eval(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lex { span, slice } => {
                write!(f, "lex error at {span:?}: `{slice}`")
            }
            Error::Parse { location, expected } => {
                write!(f, "parse error at token {location}: expected {expected}")
            }
            Error::Eval(msg) => write!(f, "eval error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
