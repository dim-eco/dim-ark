use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};

use dim_ark::dp::solve_between_traced;

use crate::ast::{Expr, Item, NodeDef, Program};
use crate::error::Error;
use crate::vm::{apply_lambda, eval_value, expect_num, num_to_i64_debug, Env, Value};

/// Per-node DP values from a `between` evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathsBetweenNode {
    pub id: String,
    pub value: i64,
    pub sum: i64,
    pub dp: i64,
}

/// Debug/trace result for `between`, including the filtered subgraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathsBetweenResult {
    pub result: i64,
    pub nodes: Vec<PathsBetweenNode>,
    pub edges: Vec<(String, String)>,
}

pub fn eval_paths_between(
    program: &Program,
    env: &Env,
    begin: i64,
    end: i64,
) -> Result<i64, Error> {
    Ok(eval_paths_between_debug(program, env, begin, end)?.result)
}

pub fn eval_paths_between_debug(
    program: &Program,
    env: &Env,
    begin: i64,
    end: i64,
) -> Result<PathsBetweenResult, Error> {
    eval_dp_between_debug(
        program,
        env,
        "paths",
        Value::node_key("", Value::Int(begin)),
        Value::node_key("", Value::Int(end)),
    )
}

pub fn eval_dp_between(
    program: &Program,
    env: &Env,
    dp_name: &str,
    begin: Value,
    end: Value,
) -> Result<i64, Error> {
    Ok(eval_dp_between_debug(program, env, dp_name, begin, end)?.result)
}

pub fn eval_dp_between_debug(
    program: &Program,
    env: &Env,
    dp_name: &str,
    begin: Value,
    end: Value,
) -> Result<PathsBetweenResult, Error> {
    let begin = normalize_node_key(begin)?;
    let end = normalize_node_key(end)?;

    let nodes_by_name = find_dp_nodes(program, dp_name)?;
    let env = RefCell::new(env.clone());

    let mut payload: HashMap<Value, Value> = HashMap::new();
    let mut edges: HashMap<Value, Vec<Value>> = HashMap::new();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(begin.clone());
    seen.insert(begin.clone());

    while let Some(node_key) = queue.pop_front() {
        let (node_name, key_payload) = split_node_key(&node_key)?;
        let node = nodes_by_name.get(node_name.as_str()).ok_or_else(|| {
            Error::Eval(format!("unknown node kind `{node_name}` in `{dp_name}`"))
        })?;

        let payload_expr = node
            .payload
            .as_ref()
            .ok_or_else(|| Error::Eval(format!("node `{node_name}` missing `payload`")))?;
        let next_expr = node
            .next
            .as_ref()
            .ok_or_else(|| Error::Eval(format!("node `{node_name}` missing `next`")))?;

        {
            let mut env = env.borrow_mut();
            env.insert("key", key_payload);

            let p = expect_num(&eval_value(payload_expr, &mut env)?)?;
            payload.insert(node_key.clone(), p);

            let next_fn = eval_value(next_expr, &mut env)?;
            let next_val = apply_lambda(&next_fn, &[], &mut env)?;
            let outs = match next_val {
                Value::List(items) => {
                    let mut keys = Vec::with_capacity(items.len());
                    for item in items {
                        keys.push(normalize_node_key(item)?);
                    }
                    keys
                }
                other => {
                    return Err(Error::Eval(format!(
                        "`next` must yield a list of node keys, got {other:?}"
                    )));
                }
            };
            for out in &outs {
                if seen.insert(out.clone()) {
                    queue.push_back(out.clone());
                }
            }
            edges.insert(node_key, outs);
        }
    }

    if !payload.contains_key(&end) {
        // End unreachable from begin via next — still allow solve_between to return zero.
        let (end_name, end_key) = split_node_key(&end)?;
        let end_node = nodes_by_name.get(end_name.as_str()).ok_or_else(|| {
            Error::Eval(format!("unknown node kind `{end_name}` in `{dp_name}`"))
        })?;
        let payload_expr = end_node
            .payload
            .as_ref()
            .ok_or_else(|| Error::Eval(format!("node `{end_name}` missing `payload`")))?;
        let mut env = env.borrow_mut();
        env.insert("key", end_key);
        let p = expect_num(&eval_value(payload_expr, &mut env)?)?;
        payload.insert(end.clone(), p);
        edges.entry(end.clone()).or_default();
    }

    let ops_node = {
        let (begin_name, _) = split_node_key(&begin)?;
        nodes_by_name.get(begin_name.as_str()).ok_or_else(|| {
            Error::Eval(format!("unknown begin node kind `{begin_name}`"))
        })?
    };

    let (unit, zero, add_fn, mul_fn) = {
        let add_expr = ops_node
            .add
            .as_ref()
            .ok_or_else(|| Error::Eval("node missing `combine`/`add`".into()))?;
        let mul_expr = ops_node
            .mul
            .as_ref()
            .ok_or_else(|| Error::Eval("node missing `extend`/`mul`".into()))?;
        let unit_expr = ops_node
            .unit
            .as_ref()
            .ok_or_else(|| Error::Eval("node missing `unit`".into()))?;
        let zero_expr = ops_node
            .zero
            .as_ref()
            .ok_or_else(|| Error::Eval("node missing `zero`".into()))?;

        let mut env = env.borrow_mut();
        let unit = expect_num(&eval_value(unit_expr, &mut env)?)?;
        let zero = expect_num(&eval_value(zero_expr, &mut env)?)?;
        let add_fn = eval_value(add_expr, &mut env)?;
        let mul_fn = eval_value(mul_expr, &mut env)?;
        (unit, zero, add_fn, mul_fn)
    };

    let err = RefCell::new(None);
    let zero_fallback = zero.clone();
    let trace = solve_between_traced(
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
                zero_fallback.clone()
            }
        },
        |a, b| match apply_binop(&mul_fn, a, b, &mut env.borrow_mut()) {
            Ok(v) => v,
            Err(e) => {
                let mut slot = err.borrow_mut();
                if slot.is_none() {
                    *slot = Some(e);
                }
                zero_fallback.clone()
            }
        },
        unit,
        zero.clone(),
    );
    if let Some(e) = err.into_inner() {
        return Err(e);
    }

    Ok(PathsBetweenResult {
        result: num_to_i64_debug(&trace.result),
        nodes: trace
            .nodes
            .into_iter()
            .map(|n| PathsBetweenNode {
                id: n.key.display_id(),
                value: num_to_i64_debug(&n.payload),
                sum: num_to_i64_debug(&n.incoming),
                dp: num_to_i64_debug(&n.dp),
            })
            .collect(),
        edges: trace
            .edges
            .into_iter()
            .map(|(from, to)| (from.display_id(), to.display_id()))
            .collect(),
    })
}

fn apply_binop(lambda: &Value, a: Value, b: Value, env: &mut Env) -> Result<Value, Error> {
    expect_num(&apply_lambda(lambda, &[a, b], env)?)
}

fn normalize_node_key(value: Value) -> Result<Value, Error> {
    match value {
        Value::NodeKey { name, key } => Ok(Value::NodeKey { name, key }),
        Value::Int(n) => Ok(Value::node_key("", Value::Int(n))),
        other => Err(Error::Eval(format!(
            "expected node key, got {other:?}"
        ))),
    }
}

fn split_node_key(value: &Value) -> Result<(String, Value), Error> {
    match value {
        Value::NodeKey { name, key } => Ok((name.clone(), (**key).clone())),
        other => Err(Error::Eval(format!(
            "expected NodeKey, got {other:?}"
        ))),
    }
}

fn find_dp_nodes<'a>(
    program: &'a Program,
    dp_name: &str,
) -> Result<HashMap<String, &'a NodeDef>, Error> {
    for item in &program.items {
        if let Item::Assign {
            name,
            value: Expr::Dp(block),
        } = item
        {
            if name == dp_name {
                let mut map = HashMap::new();
                for node in &block.nodes {
                    let key = node.name.clone().unwrap_or_default();
                    if map.insert(key.clone(), node).is_some() {
                        return Err(Error::Eval(format!(
                            "duplicate node name `{key}` in `{dp_name}`"
                        )));
                    }
                }
                if map.is_empty() {
                    return Err(Error::Eval(format!(
                        "`{dp_name}` dp block has no nodes"
                    )));
                }
                return Ok(map);
            }
        }
    }
    Err(Error::Eval(format!(
        "program has no `{dp_name}` dp assignment"
    )))
}
