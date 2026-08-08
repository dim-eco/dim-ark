/// Parse `paths.between(paths.node($begin), paths.node($end))` (whitespace-tolerant).
pub fn parse_paths_between(src: &str) -> Result<(String, String), String> {
    let s = src.trim();
    let rest = strip_prefix_name(s, "paths")?;
    let rest = strip_dot(rest)?;
    let rest = strip_prefix_name(rest, "between")?;
    let rest = strip_char(rest, '(')?;
    let rest = strip_prefix_name(rest, "paths")?;
    let rest = strip_dot(rest)?;
    let rest = strip_prefix_name(rest, "node")?;
    let rest = strip_char(rest, '(')?;
    let (begin, rest) = strip_param(rest)?;
    let rest = strip_char(rest, ')')?;
    let rest = strip_char(rest, ',')?;
    let rest = strip_prefix_name(rest, "paths")?;
    let rest = strip_dot(rest)?;
    let rest = strip_prefix_name(rest, "node")?;
    let rest = strip_char(rest, '(')?;
    let (end, rest) = strip_param(rest)?;
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

fn strip_param(s: &str) -> Result<(String, &str), String> {
    let s = skip_ws(s);
    let s = s
        .strip_prefix('$')
        .ok_or_else(|| "expected `$param`".to_string())?;
    let end = s
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(s.len());
    if end == 0 {
        return Err("expected param name after `$`".into());
    }
    Ok((s[..end].to_string(), &s[end..]))
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
        assert_eq!(b, "begin");
        assert_eq!(e, "end");
    }

    #[test]
    fn parses_whitespace() {
        let (b, e) = parse_paths_between(
            "  paths . between ( paths . node ( $x ) , paths . node ( $y ) )  ",
        )
        .unwrap();
        assert_eq!(b, "x");
        assert_eq!(e, "y");
    }

    #[test]
    fn rejects_other() {
        assert!(parse_paths_between("input.values[$nodeId] = $value").is_err());
    }
}
