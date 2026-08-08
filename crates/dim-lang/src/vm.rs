use crate::ast::Expr;
use crate::error::Error;

pub fn eval(expr: &Expr) -> Result<String, Error> {
    match expr {
        Expr::Lit(s) => Ok(s.clone()),
        Expr::Call { name, args } => {
            let values: Result<Vec<String>, Error> = args.iter().map(eval).collect();
            let values = values?;
            dispatch_intrinsic(name, &values)
        }
        other => Err(Error::Eval(format!(
            "expression not supported by eval: {other:?}"
        ))),
    }
}

fn dispatch_intrinsic(name: &str, args: &[String]) -> Result<String, Error> {
    match name {
        "__intrinsic_add" => binary_int(name, args, |a, b| a.checked_add(b)),
        "__intrinsic_sub" => binary_int(name, args, |a, b| a.checked_sub(b)),
        "__intrinsic_mul" => binary_int(name, args, |a, b| a.checked_mul(b)),
        "__intrinsic_div" => binary_int(name, args, |a, b| {
            if b == 0 {
                None
            } else {
                a.checked_div(b)
            }
        }),
        "__intrinsic_mod" => binary_int(name, args, |a, b| {
            if b == 0 {
                None
            } else {
                a.checked_rem(b)
            }
        }),
        _ => Err(Error::Eval(format!("unknown intrinsic `{name}`"))),
    }
}

fn binary_int(
    name: &str,
    args: &[String],
    op: impl FnOnce(i64, i64) -> Option<i64>,
) -> Result<String, Error> {
    if args.len() != 2 {
        return Err(Error::Eval(format!(
            "`{name}` expects 2 arguments, got {}",
            args.len()
        )));
    }
    let a = parse_i64(&args[0])?;
    let b = parse_i64(&args[1])?;
    let result = op(a, b).ok_or_else(|| Error::Eval(format!("`{name}` failed on {a} and {b}")))?;
    Ok(result.to_string())
}

fn parse_i64(s: &str) -> Result<i64, Error> {
    s.parse::<i64>()
        .map_err(|_| Error::Eval(format!("invalid int literal `{s}`")))
}
