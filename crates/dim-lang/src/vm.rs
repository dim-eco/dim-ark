use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::ast::{Expr, Stmt};
use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    NegInf,
    Str(String),
    /// Ordered id cursor for multi-item DP (`Step()` / `.current` / `.incremented()`).
    Step(i64),
    Map(BTreeMap<i64, Value>),
    List(Vec<Value>),
    Record(BTreeMap<String, Value>),
    NodeKey {
        name: String,
        key: Box<Value>,
    },
    Lambda {
        params: Vec<String>,
        body: Expr,
    },
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Int(n) => n.hash(state),
            Value::NegInf => {}
            Value::Str(s) => s.hash(state),
            Value::Step(n) => n.hash(state),
            Value::Map(map) => {
                for (k, v) in map {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Value::List(items) => {
                for item in items {
                    item.hash(state);
                }
            }
            Value::Record(fields) => {
                for (k, v) in fields {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Value::NodeKey { name, key } => {
                name.hash(state);
                key.hash(state);
            }
            Value::Lambda { .. } => {
                // Lambdas are not used as map keys; hash by discriminant only.
            }
        }
    }
}

impl Value {
    pub fn node_key(name: impl Into<String>, key: Value) -> Self {
        Value::NodeKey {
            name: name.into(),
            key: Box::new(key),
        }
    }

    pub fn display_id(&self) -> String {
        match self {
            Value::NodeKey { name, key } => {
                if name.is_empty() {
                    key.display_id()
                } else {
                    format!("{name}:{}", key.display_id())
                }
            }
            Value::Int(n) => n.to_string(),
            Value::NegInf => "-inf".into(),
            Value::Str(s) => s.clone(),
            Value::Step(n) => format!("Step({n})"),
            Value::Record(fields) if fields.is_empty() => "{}".into(),
            Value::Record(fields) => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{k}:{}", v.display_id()))
                    .collect();
                format!("{{{}}}", parts.join(","))
            }
            Value::List(items) if items.is_empty() => "[]".into(),
            other => format!("{other:?}"),
        }
    }
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
        Value::NegInf => Ok("-inf".into()),
        other => Err(Error::Eval(format!(
            "expression eval expected int, got {other:?}"
        ))),
    }
}

pub fn eval_value(expr: &Expr, env: &mut Env) -> Result<Value, Error> {
    match expr {
        Expr::Lit(s) => Ok(Value::Int(parse_i64(s)?)),
        Expr::Str(s) => Ok(Value::Str(s.clone())),
        Expr::NegInf => Ok(Value::NegInf),
        Expr::Name(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| Error::Eval(format!("unbound name `{name}`"))),
        Expr::Field { base, field } => {
            let base = eval_value(base, env)?;
            match base {
                Value::Record(fields) => fields
                    .get(field)
                    .cloned()
                    .ok_or_else(|| Error::Eval(format!("missing field `{field}`"))),
                Value::Step(n) if field == "current" => Ok(Value::Int(n)),
                other => Err(Error::Eval(format!(
                    "field access on non-record: {other:?}"
                ))),
            }
        }
        Expr::Index { base, index } => {
            let base = eval_value(base, env)?;
            let index = eval_value(index, env)?;
            let key = match index {
                Value::Int(k) => k,
                Value::Step(k) => k,
                other => {
                    return Err(Error::Eval(format!(
                        "invalid index {other:?} on {base:?}"
                    )));
                }
            };
            match base {
                Value::Map(map) => map
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| Error::Eval(format!("missing map key `{key}`"))),
                Value::List(list) => {
                    let i = usize::try_from(key).map_err(|_| {
                        Error::Eval(format!("list index out of range: {key}"))
                    })?;
                    list.get(i).cloned().ok_or_else(|| {
                        Error::Eval(format!("list index out of range: {key}"))
                    })
                }
                other => Err(Error::Eval(format!(
                    "invalid index {key} on {other:?}"
                ))),
            }
        }
        Expr::RecordLit(fields) => {
            let mut record = BTreeMap::new();
            for (name, value) in fields {
                record.insert(name.clone(), eval_value(value, env)?);
            }
            Ok(Value::Record(record))
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
                "node" => eval_node_call(arg_vals),
                "Step" => eval_step_call(arg_vals, env),
                "max" => eval_max(arg_vals),
                "__intrinsic_add" => binary_num_value(&arg_vals, num_add),
                "__intrinsic_sub" => binary_int_only(&arg_vals, |a, b| a.checked_sub(b)),
                "__intrinsic_mul" => binary_int_only(&arg_vals, |a, b| a.checked_mul(b)),
                "__intrinsic_div" => binary_int_only(&arg_vals, |a, b| {
                    if b == 0 {
                        None
                    } else {
                        a.checked_div(b)
                    }
                }),
                "__intrinsic_mod" => binary_int_only(&arg_vals, |a, b| {
                    if b == 0 {
                        None
                    } else {
                        a.checked_rem(b)
                    }
                }),
                "__intrinsic_lt" => cmp_ints(&arg_vals, |a, b| a < b),
                "__intrinsic_le" => cmp_ints(&arg_vals, |a, b| a <= b),
                "__intrinsic_gt" => cmp_ints(&arg_vals, |a, b| a > b),
                "__intrinsic_ge" => cmp_ints(&arg_vals, |a, b| a >= b),
                "__intrinsic_eq" => cmp_eq(&arg_vals),
                "__intrinsic_ne" => match cmp_eq(&arg_vals)? {
                    Value::Int(0) => Ok(Value::Int(1)),
                    Value::Int(1) => Ok(Value::Int(0)),
                    other => Ok(other),
                },
                other => Err(Error::Eval(format!("unknown call `{other}`"))),
            }
        }
        Expr::MethodCall { base, method, args } => {
            // `node.main(...)` / `node.result()` sugar → named NodeKey
            if let Expr::Name(name) = base.as_ref() {
                if name == "node" {
                    let mut node_args = vec![Value::Str(method.clone())];
                    if args.is_empty() {
                        node_args.push(Value::Record(BTreeMap::new()));
                    } else {
                        for a in args {
                            node_args.push(eval_value(a, env)?);
                        }
                    }
                    return eval_node_call(node_args);
                }
            }

            let base_v = eval_value(base, env)?;
            let arg_vals: Result<Vec<Value>, Error> =
                args.iter().map(|a| eval_value(a, env)).collect();
            let arg_vals = arg_vals?;
            eval_method(base_v, method, arg_vals)
        }
        Expr::Dp(_) => Err(Error::Eval(
            "dp blocks are not evaluated as expressions".into(),
        )),
    }
}

fn eval_step_call(args: Vec<Value>, env: &Env) -> Result<Value, Error> {
    match args.len() {
        0 => {
            let input = env
                .get("input")
                .ok_or_else(|| Error::Eval("`Step()` requires `input` in scope".into()))?;
            let Value::Record(fields) = input else {
                return Err(Error::Eval("`Step()` expected record `input`".into()));
            };
            let first = fields.get("first_id").ok_or_else(|| {
                Error::Eval("`Step()` expected `input.first_id`".into())
            })?;
            Ok(Value::Step(expect_int(first)?))
        }
        1 => Ok(Value::Step(expect_int(&args[0])?)),
        n => Err(Error::Eval(format!(
            "`Step` expects 0 or 1 arguments, got {n}"
        ))),
    }
}

fn eval_method(base: Value, method: &str, args: Vec<Value>) -> Result<Value, Error> {
    match (base, method, args.as_slice()) {
        (Value::Step(n), "incremented", []) => Ok(Value::Step(n + 1)),
        (Value::Record(fields), "incremented", []) => {
            let id = fields
                .get("id")
                .ok_or_else(|| Error::Eval("`incremented()` on record missing `id`".into()))?;
            match id {
                Value::Step(n) => Ok(Value::Step(n + 1)),
                Value::Int(n) => Ok(Value::Int(n + 1)),
                other => Err(Error::Eval(format!(
                    "`incremented()` expected Step/Int id, got {other:?}"
                ))),
            }
        }
        (base, method, args) => Err(Error::Eval(format!(
            "unknown method `{method}` on {base:?} with {} args",
            args.len()
        ))),
    }
}

fn eval_node_call(args: Vec<Value>) -> Result<Value, Error> {
    match args.len() {
        1 => Ok(Value::node_key("", args.into_iter().next().unwrap())),
        2 => {
            let mut iter = args.into_iter();
            let name_v = iter.next().unwrap();
            let key_v = iter.next().unwrap();
            let Value::Str(name) = name_v else {
                return Err(Error::Eval(format!(
                    "`node` name must be a string, got {name_v:?}"
                )));
            };
            // `node('sink', {})` parses `{}` as an empty block → List([]).
            let key = match key_v {
                Value::List(items) if items.is_empty() => Value::Record(BTreeMap::new()),
                other => other,
            };
            Ok(Value::node_key(name, key))
        }
        n => Err(Error::Eval(format!(
            "`node` expects 1 or 2 arguments, got {n}"
        ))),
    }
}

fn eval_max(args: Vec<Value>) -> Result<Value, Error> {
    if args.len() != 2 {
        return Err(Error::Eval(format!(
            "`max` expects 2 arguments, got {}",
            args.len()
        )));
    }
    Ok(num_max(args[0].clone(), args[1].clone()))
}

fn num_max(a: Value, b: Value) -> Value {
    match (&a, &b) {
        (Value::NegInf, _) => b,
        (_, Value::NegInf) => a,
        (Value::Int(x), Value::Int(y)) => Value::Int((*x).max(*y)),
        _ => a,
    }
}

fn num_add(a: &Value, b: &Value) -> Result<Value, Error> {
    match (a, b) {
        (Value::NegInf, _) | (_, Value::NegInf) => Ok(Value::NegInf),
        (Value::Int(x), Value::Int(y)) => x
            .checked_add(*y)
            .map(Value::Int)
            .ok_or_else(|| Error::Eval(format!("add overflow on {x} and {y}"))),
        _ => Err(Error::Eval(format!("add expected ints, got {a:?} and {b:?}"))),
    }
}

fn binary_num_value(
    args: &[Value],
    op: impl FnOnce(&Value, &Value) -> Result<Value, Error>,
) -> Result<Value, Error> {
    if args.len() != 2 {
        return Err(Error::Eval(format!(
            "binary op expects 2 arguments, got {}",
            args.len()
        )));
    }
    op(&args[0], &args[1])
}

fn binary_int_only(
    args: &[Value],
    op: impl FnOnce(i64, i64) -> Option<i64>,
) -> Result<Value, Error> {
    if args.len() != 2 {
        return Err(Error::Eval(format!(
            "binary op expects 2 arguments, got {}",
            args.len()
        )));
    }
    let a = expect_int(&args[0])?;
    let b = expect_int(&args[1])?;
    op(a, b)
        .map(Value::Int)
        .ok_or_else(|| Error::Eval(format!("arithmetic failed on {a} and {b}")))
}

fn cmp_ints(args: &[Value], op: impl FnOnce(i64, i64) -> bool) -> Result<Value, Error> {
    if args.len() != 2 {
        return Err(Error::Eval(format!(
            "compare expects 2 arguments, got {}",
            args.len()
        )));
    }
    let a = expect_int(&args[0])?;
    let b = expect_int(&args[1])?;
    Ok(Value::Int(if op(a, b) { 1 } else { 0 }))
}

fn cmp_eq(args: &[Value]) -> Result<Value, Error> {
    if args.len() != 2 {
        return Err(Error::Eval(format!(
            "compare expects 2 arguments, got {}",
            args.len()
        )));
    }
    Ok(Value::Int(if args[0] == args[1] { 1 } else { 0 }))
}

pub fn apply_lambda(lambda: &Value, args: &[Value], env: &mut Env) -> Result<Value, Error> {
    let Value::Lambda { params, body } = lambda else {
        return Err(Error::Eval(format!("expected lambda, got {lambda:?}")));
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

fn eval_stmts(stmts: &[Stmt], env: &mut Env, yields: &mut Vec<Value>) -> Result<(), Error> {
    for stmt in stmts {
        match stmt {
            Stmt::Yield(expr) => {
                yields.push(eval_value(expr, env)?);
            }
            Stmt::Expr(expr) => {
                let _ = eval_value(expr, env)?;
            }
            Stmt::If {
                cond,
                body,
                else_body,
            } => {
                let cond_val = eval_value(cond, env)?;
                if is_truthy(&cond_val)? {
                    eval_stmts(body, env, yields)?;
                } else if let Some(else_body) = else_body {
                    eval_stmts(else_body, env, yields)?;
                }
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

fn is_truthy(value: &Value) -> Result<bool, Error> {
    match value {
        Value::Int(n) => Ok(*n != 0),
        other => Err(Error::Eval(format!(
            "condition expected int, got {other:?}"
        ))),
    }
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

/// Numeric DP payload/result: ints, with NegInf for the zero of max-plus.
pub fn expect_num(value: &Value) -> Result<Value, Error> {
    match value {
        Value::Int(_) | Value::NegInf => Ok(value.clone()),
        other => Err(Error::Eval(format!(
            "expected numeric value, got {other:?}"
        ))),
    }
}

pub fn num_to_i64_debug(value: &Value) -> i64 {
    match value {
        Value::Int(n) => *n,
        Value::NegInf => i64::MIN,
        _ => 0,
    }
}
