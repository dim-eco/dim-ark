use std::collections::BTreeMap;

use crate::ast::{Expr, Stmt};
use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Map(BTreeMap<i64, Value>),
    List(Vec<Value>),
    Record(BTreeMap<String, Value>),
    Lambda { params: Vec<String>, body: Expr },
}

#[derive(Debug, Clone, Default)]
pub struct Env {
    bindings: BTreeMap<String, Value>,
}

impl Env {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.bindings.get(name)
    }

    pub fn insert(&mut self, name: impl Into<String>, value: Value) {
        self.bindings.insert(name.into(), value);
    }
}

pub fn set_data(env: &mut Env, name: &str, value: Value) {
    env.insert(name, value);
}

/// Expression-only eval used by `eval_src` (no environment).
pub fn eval(expr: &Expr) -> Result<String, Error> {
    let mut env = Env::new();
    match eval_value(expr, &mut env)? {
        Value::Int(n) => Ok(n.to_string()),
        other => Err(Error::Eval(format!(
            "expression eval expected int, got {other:?}"
        ))),
    }
}

pub fn eval_value(expr: &Expr, env: &mut Env) -> Result<Value, Error> {
    match expr {
        Expr::Lit(s) => Ok(Value::Int(parse_i64(s)?)),
        Expr::Name(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| Error::Eval(format!("unbound name `{name}`"))),
        Expr::Field { base, field } => {
            let base = eval_value(base, env)?;
            match base {
                Value::Record(fields) => fields.get(field).cloned().ok_or_else(|| {
                    Error::Eval(format!("missing field `{field}`"))
                }),
                other => Err(Error::Eval(format!(
                    "field access on non-record: {other:?}"
                ))),
            }
        }
        Expr::Index { base, index } => {
            let base = eval_value(base, env)?;
            let index = eval_value(index, env)?;
            match (base, index) {
                (Value::Map(map), Value::Int(k)) => map
                    .get(&k)
                    .cloned()
                    .ok_or_else(|| Error::Eval(format!("missing map key `{k}`"))),
                (Value::List(list), Value::Int(i)) => {
                    let i = usize::try_from(i).map_err(|_| {
                        Error::Eval(format!("list index out of range: {i}"))
                    })?;
                    list.get(i).cloned().ok_or_else(|| {
                        Error::Eval(format!("list index out of range: {i}"))
                    })
                }
                (base, index) => Err(Error::Eval(format!(
                    "invalid index {index:?} on {base:?}"
                ))),
            }
        }
        Expr::Lambda { params, body } => Ok(Value::Lambda {
            params: params.clone(),
            body: (**body).clone(),
        }),
        Expr::Block(stmts) => {
            let mut yields = Vec::new();
            eval_stmts(stmts, env, &mut yields)?;
            Ok(Value::List(yields))
        }
        Expr::Call { name, args } => {
            let arg_vals: Result<Vec<Value>, Error> =
                args.iter().map(|a| eval_value(a, env)).collect();
            let arg_vals = arg_vals?;
            match name.as_str() {
                "node" => {
                    if arg_vals.len() != 1 {
                        return Err(Error::Eval(format!(
                            "`node` expects 1 argument, got {}",
                            arg_vals.len()
                        )));
                    }
                    Ok(arg_vals.into_iter().next().unwrap())
                }
                "__intrinsic_add"
                | "__intrinsic_sub"
                | "__intrinsic_mul"
                | "__intrinsic_div"
                | "__intrinsic_mod" => {
                    let strings: Result<Vec<String>, Error> = arg_vals
                        .iter()
                        .map(|v| match v {
                            Value::Int(n) => Ok(n.to_string()),
                            other => Err(Error::Eval(format!(
                                "intrinsic expected int, got {other:?}"
                            ))),
                        })
                        .collect();
                    let result = dispatch_intrinsic(name, &strings?)?;
                    Ok(Value::Int(parse_i64(&result)?))
                }
                other => Err(Error::Eval(format!("unknown call `{other}`"))),
            }
        }
        Expr::Dp(_) => Err(Error::Eval(
            "dp blocks are not evaluated as expressions".into(),
        )),
    }
}

pub fn apply_lambda(lambda: &Value, args: &[Value], env: &mut Env) -> Result<Value, Error> {
    let Value::Lambda { params, body } = lambda else {
        return Err(Error::Eval(format!(
            "expected lambda, got {lambda:?}"
        )));
    };
    if params.len() != args.len() {
        return Err(Error::Eval(format!(
            "lambda expects {} args, got {}",
            params.len(),
            args.len()
        )));
    }

    let saved: Vec<(String, Option<Value>)> = params
        .iter()
        .map(|p| (p.clone(), env.get(p).cloned()))
        .collect();

    for (param, arg) in params.iter().zip(args.iter()) {
        env.insert(param.clone(), arg.clone());
    }

    let result = eval_value(body, env);

    for (param, prev) in saved {
        match prev {
            Some(v) => env.insert(param, v),
            None => {
                env.bindings.remove(&param);
            }
        }
    }

    result
}

fn eval_stmts(
    stmts: &[Stmt],
    env: &mut Env,
    yields: &mut Vec<Value>,
) -> Result<(), Error> {
    for stmt in stmts {
        match stmt {
            Stmt::Yield(expr) => {
                yields.push(eval_value(expr, env)?);
            }
            Stmt::Expr(expr) => {
                let _ = eval_value(expr, env)?;
            }
            Stmt::For { var, iter, body } => {
                let iter_val = eval_value(iter, env)?;
                let items = match iter_val {
                    Value::List(items) => items,
                    other => {
                        return Err(Error::Eval(format!(
                            "for-loop expected list, got {other:?}"
                        )));
                    }
                };
                let saved = env.get(var).cloned();
                for item in items {
                    env.insert(var.clone(), item);
                    eval_stmts(body, env, yields)?;
                }
                match saved {
                    Some(v) => env.insert(var.clone(), v),
                    None => {
                        env.bindings.remove(var);
                    }
                }
            }
        }
    }
    Ok(())
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

pub fn expect_int(value: &Value) -> Result<i64, Error> {
    match value {
        Value::Int(n) => Ok(*n),
        other => Err(Error::Eval(format!("expected int, got {other:?}"))),
    }
}
