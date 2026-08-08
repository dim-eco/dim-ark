/// Argument to `paths.node(...)`: `$param` or a literal integer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BetweenArg {
    Param(String),
    Lit(i64),
}

/// Parse `paths.between(paths.node($begin|$n), paths.node($end|$m))` (whitespace-tolerant).
pub fn parse_paths_between(src: &str) -> Result<(BetweenArg, BetweenArg), String> {
    let s = src.trim();
    let rest = strip_prefix_name(s, "paths")?;
    let rest = strip_dot(rest)?;
    let rest = strip_prefix_name(rest, "between")?;
    let rest = strip_char(rest, '(')?;
    let rest = strip_prefix_name(rest, "paths")?;
    let rest = strip_dot(rest)?;
    let rest = strip_prefix_name(rest, "node")?;
    let rest = strip_char(rest, '(')?;
    let (begin, rest) = strip_arg(rest)?;
    let rest = strip_char(rest, ')')?;
    let rest = strip_char(rest, ',')?;
    let rest = strip_prefix_name(rest, "paths")?;
    let rest = strip_dot(rest)?;
    let rest = strip_prefix_name(rest, "node")?;
    let rest = strip_char(rest, '(')?;
    let (end, rest) = strip_arg(rest)?;
    let rest = strip_char(rest, ')')?;
    let rest = strip_char(rest, ')')?;
    if !rest.trim().is_empty() {
        return Err("unexpected trailing input in frag".into());
    }
    Ok((begin, end))
}

fn skip_ws(s: &str) -> &str {
    s.trim_start()
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

fn strip_arg(s: &str) -> Result<(BetweenArg, &str), String> {
    let s = skip_ws(s);
    if let Some(rest) = s.strip_prefix('$') {
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if end == 0 {
            return Err("expected param name after `$`".into());
        }
        return Ok((BetweenArg::Param(rest[..end].to_string()), &rest[end..]));
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
        return Err("expected `$param` or integer literal".into());
    }
    let n: i64 = digits[..end]
        .parse()
        .map_err(|_| "invalid integer literal".to_string())?;
    let value = if neg { -n } else { n };
    Ok((BetweenArg::Lit(value), &digits[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical() {
        let (b, e) = parse_paths_between(
            "paths.between(paths.node($begin), paths.node($end))",
        )
        .unwrap();
        assert_eq!(b, BetweenArg::Param("begin".into()));
        assert_eq!(e, BetweenArg::Param("end".into()));
    }

    #[test]
    fn parses_literals() {
        let (b, e) =
            parse_paths_between("paths.between(paths.node(1), paths.node(9))").unwrap();
        assert_eq!(b, BetweenArg::Lit(1));
        assert_eq!(e, BetweenArg::Lit(9));
    }

    #[test]
    fn parses_mixed() {
        let (b, e) =
            parse_paths_between("paths.between(paths.node(1), paths.node($end))").unwrap();
        assert_eq!(b, BetweenArg::Lit(1));
        assert_eq!(e, BetweenArg::Param("end".into()));
    }

    #[test]
    fn parses_whitespace() {
        let (b, e) = parse_paths_between(
            "  paths . between ( paths . node ( $x ) , paths . node ( $y ) )  ",
        )
        .unwrap();
        assert_eq!(b, BetweenArg::Param("x".into()));
        assert_eq!(e, BetweenArg::Param("y".into()));
    }

    #[test]
    fn rejects_other() {
        assert!(parse_paths_between("input.values[$nodeId] = $value").is_err());
    }
}
