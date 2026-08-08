use std::collections::BTreeMap;

use dim_lang::{Env, Value};

/// Scalar field in a node key record: `$param`, integer literal, or `Step()` / `Step(n)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldArg {
    Param(String),
    Lit(i64),
    /// `Step()` — resolved from `input.first_id` (or `$first_id` binding).
    Step,
    /// `Step(n)` / `Step($param)`
    StepOf(Box<FieldArg>),
}

/// Endpoint of `dp.between(...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeEndpoint {
    /// `paths.node(1)` / `paths.node($begin)` — unnamed node, int key.
    Simple(FieldArg),
    /// `backpack.node('main', {weight: 0, id: Step()})` or `node.main({...})`
    Named {
        name: String,
        fields: Vec<(String, FieldArg)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetweenFrag {
    pub dp_name: String,
    pub begin: NodeEndpoint,
    pub end: NodeEndpoint,
}

/// Parse `$dp.between(...)` (whitespace-tolerant).
///
/// Endpoints may be:
/// - `paths.node(1)` / `paths.node($begin)`
/// - `backpack.node('main', {weight: 0, id: Step()})`
/// - `node.main({weight: 0, id: Step()})` / `node.result()`
pub fn parse_between_frag(src: &str) -> Result<BetweenFrag, String> {
    let s = src.trim();
    let (dp_name, rest) = strip_ident(s)?;
    let rest = strip_dot(rest)?;
    let rest = strip_prefix_name(rest, "between")?;
    let rest = strip_char(rest, '(')?;
    let (begin, rest) = strip_node_call(rest, &dp_name)?;
    let rest = strip_char(rest, ',')?;
    let (end, rest) = strip_node_call(rest, &dp_name)?;
    let rest = strip_char(rest, ')')?;
    if !rest.trim().is_empty() {
        return Err("unexpected trailing input in frag".into());
    }
    Ok(BetweenFrag {
        dp_name,
        begin,
        end,
    })
}

pub fn resolve_endpoint(
    endpoint: &NodeEndpoint,
    bindings: &std::collections::HashMap<String, i64>,
    env: &Env,
) -> Result<Value, String> {
    match endpoint {
        NodeEndpoint::Simple(arg) => {
            let n = resolve_int_field(arg, bindings, env)?;
            Ok(Value::node_key("", Value::Int(n)))
        }
        NodeEndpoint::Named { name, fields } => {
            let mut record = BTreeMap::new();
            for (field, arg) in fields {
                record.insert(field.clone(), resolve_value_field(arg, bindings, env)?);
            }
            Ok(Value::node_key(name.clone(), Value::Record(record)))
        }
    }
}

pub fn endpoint_is_complete(endpoint: &NodeEndpoint) -> bool {
    match endpoint {
        NodeEndpoint::Simple(arg) => field_is_complete(arg),
        NodeEndpoint::Named { fields, .. } => fields.iter().all(|(_, a)| field_is_complete(a)),
    }
}

fn field_is_complete(arg: &FieldArg) -> bool {
    match arg {
        FieldArg::Lit(_) | FieldArg::Step => true,
        FieldArg::Param(_) => false,
        FieldArg::StepOf(inner) => field_is_complete(inner),
    }
}

fn resolve_int_field(
    arg: &FieldArg,
    bindings: &std::collections::HashMap<String, i64>,
    env: &Env,
) -> Result<i64, String> {
    match arg {
        FieldArg::Lit(v) => Ok(*v),
        FieldArg::Param(name) => bindings
            .get(name)
            .copied()
            .ok_or_else(|| format!("missing binding `{name}`")),
        FieldArg::Step | FieldArg::StepOf(_) => match resolve_value_field(arg, bindings, env)? {
            Value::Step(n) | Value::Int(n) => Ok(n),
            other => Err(format!("expected int/Step, got {other:?}")),
        },
    }
}

fn resolve_value_field(
    arg: &FieldArg,
    bindings: &std::collections::HashMap<String, i64>,
    env: &Env,
) -> Result<Value, String> {
    match arg {
        FieldArg::Lit(v) => Ok(Value::Int(*v)),
        FieldArg::Param(name) => bindings
            .get(name)
            .copied()
            .map(Value::Int)
            .ok_or_else(|| format!("missing binding `{name}`")),
        FieldArg::Step => Ok(Value::Step(resolve_first_id(bindings, env)?)),
        FieldArg::StepOf(inner) => {
            Ok(Value::Step(resolve_int_field(inner, bindings, env)?))
        }
    }
}

fn resolve_first_id(
    bindings: &std::collections::HashMap<String, i64>,
    env: &Env,
) -> Result<i64, String> {
    if let Some(n) = bindings.get("first_id") {
        return Ok(*n);
    }
    let input = env
        .get("input")
        .ok_or_else(|| "`Step()` requires `input` data or `first_id` binding".to_string())?;
    let Value::Record(fields) = input else {
        return Err("`Step()` expected record `input`".into());
    };
    let first = fields
        .get("first_id")
        .ok_or_else(|| "`Step()` expected `input.first_id`".to_string())?;
    match first {
        Value::Int(n) => Ok(*n),
        other => Err(format!("`input.first_id` expected int, got {other:?}")),
    }
}

fn skip_ws(s: &str) -> &str {
    s.trim_start()
}

fn strip_ident(s: &str) -> Result<(String, &str), String> {
    let s = skip_ws(s);
    let end = s
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(s.len());
    if end == 0 {
        return Err("expected identifier".into());
    }
    Ok((s[..end].to_string(), &s[end..]))
}

fn strip_prefix_name<'a>(s: &'a str, name: &str) -> Result<&'a str, String> {
    let s = skip_ws(s);
    if let Some(rest) = s.strip_prefix(name) {
        if rest
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_')
        {
            return Ok(rest);
        }
    }
    Err(format!("expected `{name}`"))
}

fn strip_dot(s: &str) -> Result<&str, String> {
    let s = skip_ws(s);
    s.strip_prefix('.')
        .ok_or_else(|| "expected `.`".to_string())
}

fn strip_char(s: &str, ch: char) -> Result<&str, String> {
    let s = skip_ws(s);
    s.strip_prefix(ch)
        .ok_or_else(|| format!("expected `{ch}`"))
}

fn strip_node_call<'a>(s: &'a str, dp_name: &str) -> Result<(NodeEndpoint, &'a str), String> {
    let s = skip_ws(s);

    // `node.main({...})` / `node.result()`
    if let Ok(rest) = strip_prefix_name(s, "node") {
        let rest = skip_ws(rest);
        if rest.starts_with('.') {
            let rest = strip_dot(rest)?;
            let (name, rest) = strip_ident(rest)?;
            let rest = strip_char(rest, '(')?;
            let rest = skip_ws(rest);
            if rest.starts_with(')') {
                let rest = strip_char(rest, ')')?;
                return Ok((
                    NodeEndpoint::Named {
                        name,
                        fields: Vec::new(),
                    },
                    rest,
                ));
            }
            let (fields, rest) = strip_record(rest)?;
            let rest = strip_char(rest, ')')?;
            return Ok((NodeEndpoint::Named { name, fields }, rest));
        }
    }

    // `dp.node(...)`
    let rest = strip_prefix_name(s, dp_name)?;
    let rest = strip_dot(rest)?;
    let rest = strip_prefix_name(rest, "node")?;
    let rest = skip_ws(rest);

    // `dp.node.main({...})`
    if rest.starts_with('.') {
        let rest = strip_dot(rest)?;
        let (name, rest) = strip_ident(rest)?;
        let rest = strip_char(rest, '(')?;
        let rest = skip_ws(rest);
        if rest.starts_with(')') {
            let rest = strip_char(rest, ')')?;
            return Ok((
                NodeEndpoint::Named {
                    name,
                    fields: Vec::new(),
                },
                rest,
            ));
        }
        let (fields, rest) = strip_record(rest)?;
        let rest = strip_char(rest, ')')?;
        return Ok((NodeEndpoint::Named { name, fields }, rest));
    }

    let rest = strip_char(rest, '(')?;
    let rest = skip_ws(rest);

    // Named: 'cell', { ... }  OR simple: $param / int
    if rest.starts_with('\'') {
        let (name, rest) = strip_string(rest)?;
        let rest = strip_char(rest, ',')?;
        let (fields, rest) = strip_record(rest)?;
        let rest = strip_char(rest, ')')?;
        Ok((NodeEndpoint::Named { name, fields }, rest))
    } else {
        let (arg, rest) = strip_field_arg(rest)?;
        let rest = strip_char(rest, ')')?;
        Ok((NodeEndpoint::Simple(arg), rest))
    }
}

fn strip_string(s: &str) -> Result<(String, &str), String> {
    let s = skip_ws(s);
    let rest = s
        .strip_prefix('\'')
        .ok_or_else(|| "expected string literal".to_string())?;
    let mut out = String::new();
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some((_, n)) => out.push(n),
                None => return Err("invalid string escape".into()),
            }
        } else if c == '\'' {
            return Ok((out, &rest[i + 1..]));
        } else {
            out.push(c);
        }
    }
    Err("unterminated string literal".into())
}

fn strip_record(s: &str) -> Result<(Vec<(String, FieldArg)>, &str), String> {
    let rest = strip_char(s, '{')?;
    let rest = skip_ws(rest);
    if let Some(rest) = rest.strip_prefix('}') {
        return Ok((Vec::new(), rest));
    }

    let mut fields = Vec::new();
    let mut rest = rest;
    loop {
        let (name, next) = strip_ident(rest)?;
        let next = strip_char(next, ':')?;
        let (arg, next) = strip_field_arg(next)?;
        fields.push((name, arg));
        let next = skip_ws(next);
        if let Some(next) = next.strip_prefix(',') {
            rest = skip_ws(next);
            continue;
        }
        let next = strip_char(next, '}')?;
        return Ok((fields, next));
    }
}

fn strip_field_arg(s: &str) -> Result<(FieldArg, &str), String> {
    let s = skip_ws(s);

    // Step() / Step(n) / Step($param)
    if let Ok(rest) = strip_prefix_name(s, "Step") {
        let rest = strip_char(rest, '(')?;
        let rest = skip_ws(rest);
        if rest.starts_with(')') {
            let rest = strip_char(rest, ')')?;
            return Ok((FieldArg::Step, rest));
        }
        let (inner, rest) = strip_field_arg(rest)?;
        let rest = strip_char(rest, ')')?;
        return Ok((FieldArg::StepOf(Box::new(inner)), rest));
    }

    if let Some(rest) = s.strip_prefix('$') {
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if end == 0 {
            return Err("expected param name after `$`".into());
        }
        return Ok((FieldArg::Param(rest[..end].to_string()), &rest[end..]));
    }

    let (neg, digits) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (false, s)
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 {
        return Err("expected `$param`, integer literal, or `Step(...)`".into());
    }
    let n: i64 = digits[..end]
        .parse()
        .map_err(|_| "invalid integer literal".to_string())?;
    let value = if neg { -n } else { n };
    Ok((FieldArg::Lit(value), &digits[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical() {
        let frag = parse_between_frag(
            "paths.between(paths.node($begin), paths.node($end))",
        )
        .unwrap();
        assert_eq!(frag.dp_name, "paths");
        assert_eq!(frag.begin, NodeEndpoint::Simple(FieldArg::Param("begin".into())));
        assert_eq!(frag.end, NodeEndpoint::Simple(FieldArg::Param("end".into())));
    }

    #[test]
    fn parses_literals() {
        let frag =
            parse_between_frag("paths.between(paths.node(1), paths.node(9))").unwrap();
        assert_eq!(frag.begin, NodeEndpoint::Simple(FieldArg::Lit(1)));
        assert_eq!(frag.end, NodeEndpoint::Simple(FieldArg::Lit(9)));
    }

    #[test]
    fn parses_mixed() {
        let frag =
            parse_between_frag("paths.between(paths.node(1), paths.node($end))").unwrap();
        assert_eq!(frag.begin, NodeEndpoint::Simple(FieldArg::Lit(1)));
        assert_eq!(frag.end, NodeEndpoint::Simple(FieldArg::Param("end".into())));
    }

    #[test]
    fn parses_whitespace() {
        let frag = parse_between_frag(
            "  paths . between ( paths . node ( $x ) , paths . node ( $y ) )  ",
        )
        .unwrap();
        assert_eq!(frag.begin, NodeEndpoint::Simple(FieldArg::Param("x".into())));
        assert_eq!(frag.end, NodeEndpoint::Simple(FieldArg::Param("y".into())));
    }

    #[test]
    fn parses_backpack_string_style() {
        let frag = parse_between_frag(
            "backpack.between(backpack.node('main', {weight: 0, id: Step()}), backpack.node('result', {}))",
        )
        .unwrap();
        assert_eq!(frag.dp_name, "backpack");
        assert_eq!(
            frag.begin,
            NodeEndpoint::Named {
                name: "main".into(),
                fields: vec![
                    ("weight".into(), FieldArg::Lit(0)),
                    ("id".into(), FieldArg::Step),
                ],
            }
        );
        assert_eq!(
            frag.end,
            NodeEndpoint::Named {
                name: "result".into(),
                fields: vec![],
            }
        );
    }

    #[test]
    fn parses_backpack_method_style() {
        let frag = parse_between_frag(
            "backpack.between(node.main({weight: 0, id: Step()}), node.result())",
        )
        .unwrap();
        assert_eq!(
            frag.begin,
            NodeEndpoint::Named {
                name: "main".into(),
                fields: vec![
                    ("weight".into(), FieldArg::Lit(0)),
                    ("id".into(), FieldArg::Step),
                ],
            }
        );
        assert_eq!(
            frag.end,
            NodeEndpoint::Named {
                name: "result".into(),
                fields: vec![],
            }
        );
        assert!(endpoint_is_complete(&frag.begin));
        assert!(endpoint_is_complete(&frag.end));
    }

    #[test]
    fn rejects_trailing() {
        assert!(parse_between_frag("paths.between(paths.node(1), paths.node(9))x").is_err());
    }
}
