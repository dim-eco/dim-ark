use std::cell::RefCell;
use std::collections::HashMap;

use dim_ark::dp::solve_between;

use crate::ast::{Expr, Item, NodeDef, Program};
use crate::error::Error;
use crate::vm::{apply_lambda, eval_value, expect_int, Env, Value};

pub fn eval_paths_between(
    program: &Program,
    env: &Env,
    begin: i64,
    end: i64,
) -> Result<i64, Error> {
    let node = find_paths_node(program)?;
    let env = RefCell::new(env.clone());

    let input = env
        .borrow()
        .get("input")
        .cloned()
        .ok_or_else(|| Error::Eval("missing data `input`".into()))?;
    let values = match &input {
        Value::Record(fields) => fields
            .get("values")
            .ok_or_else(|| Error::Eval("input missing `values`".into()))?,
        other => {
            return Err(Error::Eval(format!(
                "`input` must be a record, got {other:?}"
            )));
        }
    };
    let Value::Map(values_map) = values else {
        return Err(Error::Eval(format!(
            "`input.values` must be a map, got {values:?}"
        )));
    };

    let keys: Vec<i64> = values_map.keys().copied().collect();

    let payload_expr = node
        .payload
        .as_ref()
        .ok_or_else(|| Error::Eval("paths node missing `payload`".into()))?;
    let next_expr = node
        .next
        .as_ref()
        .ok_or_else(|| Error::Eval("paths node missing `next`".into()))?;
    let add_expr = node
        .add
        .as_ref()
        .ok_or_else(|| Error::Eval("paths node missing `combine`/`add`".into()))?;
    let mul_expr = node
        .mul
        .as_ref()
        .ok_or_else(|| Error::Eval("paths node missing `extend`/`mul`".into()))?;
    let unit_expr = node
        .unit
        .as_ref()
        .ok_or_else(|| Error::Eval("paths node missing `unit`".into()))?;
    let zero_expr = node
        .zero
        .as_ref()
        .ok_or_else(|| Error::Eval("paths node missing `zero`".into()))?;

    let mut payload = HashMap::new();
    let mut edges = HashMap::new();

    for key in &keys {
        let mut env = env.borrow_mut();
        env.insert("key", Value::Int(*key));

        let p = expect_int(&eval_value(payload_expr, &mut env)?)?;
        payload.insert(*key, p);

        let next_fn = eval_value(next_expr, &mut env)?;
        let next_val = apply_lambda(&next_fn, &[], &mut env)?;
        let outs = match next_val {
            Value::List(items) => {
                let mut ids = Vec::with_capacity(items.len());
                for item in items {
                    ids.push(expect_int(&item)?);
                }
                ids
            }
            other => {
                return Err(Error::Eval(format!(
                    "`next` must yield a list of ids, got {other:?}"
                )));
            }
        };
        edges.insert(*key, outs);
    }

    let (unit, zero, add_fn, mul_fn) = {
        let mut env = env.borrow_mut();
        let unit = expect_int(&eval_value(unit_expr, &mut env)?)?;
        let zero = expect_int(&eval_value(zero_expr, &mut env)?)?;
        let add_fn = eval_value(add_expr, &mut env)?;
        let mul_fn = eval_value(mul_expr, &mut env)?;
        (unit, zero, add_fn, mul_fn)
    };

    let err = RefCell::new(None);
    let result = solve_between(
        &edges,
        &payload,
        &begin,
        &end,
        |a, b| match apply_binop(&add_fn, a, b, &mut env.borrow_mut()) {
            Ok(v) => v,
            Err(e) => {
                let mut slot = err.borrow_mut();
                if slot.is_none() {
                    *slot = Some(e);
                }
                zero
            }
        },
        |a, b| match apply_binop(&mul_fn, a, b, &mut env.borrow_mut()) {
            Ok(v) => v,
            Err(e) => {
                let mut slot = err.borrow_mut();
                if slot.is_none() {
                    *slot = Some(e);
                }
                zero
            }
        },
        unit,
        zero,
    );
    if let Some(e) = err.into_inner() {
        return Err(e);
    }
    Ok(result)
}

fn apply_binop(lambda: &Value, a: i64, b: i64, env: &mut Env) -> Result<i64, Error> {
    expect_int(&apply_lambda(
        lambda,
        &[Value::Int(a), Value::Int(b)],
        env,
    )?)
}

fn find_paths_node(program: &Program) -> Result<&NodeDef, Error> {
    for item in &program.items {
        if let Item::Assign {
            name,
            value: Expr::Dp(block),
        } = item
        {
            if name == "paths" {
                return block.nodes.first().ok_or_else(|| {
                    Error::Eval("paths dp block has no nodes".into())
                });
            }
        }
    }
    Err(Error::Eval("program has no `paths` dp assignment".into()))
}
