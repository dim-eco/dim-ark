use logos::Logos;

use crate::ast::{c, DpBlock, Expr, Item, NodeDef, Program, Stmt, Type};
use crate::error::Error;
use crate::token::{Spanned, Token};

pub fn lex(src: &str) -> Result<Vec<Spanned>, Error> {
    let mut tokens = Vec::new();
    let mut lexer = Token::lexer(src);
    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => {
                let span = lexer.span();
                tokens.push(Spanned {
                    token,
                    start: span.start,
                    end: span.end,
                });
            }
            Err(()) => {
                return Err(Error::Lex {
                    span: lexer.span(),
                    slice: lexer.slice().to_owned(),
                });
            }
        }
    }
    Ok(tokens)
}

fn map_parse_err(err: peg::error::ParseError<usize>) -> Error {
    Error::Parse {
        location: err.location,
        expected: format!("{:?}", err.expected),
    }
}

pub fn parse(src: &str) -> Result<Program, Error> {
    let tokens = lex(src)?;
    dim_parser::program(&tokens, src).map_err(map_parse_err)
}

pub fn parse_expr(src: &str) -> Result<Expr, Error> {
    let tokens = lex(src)?;
    dim_parser::expr(&tokens, src).map_err(map_parse_err)
}

fn apply_node_field(node: &mut NodeDef, name: &str, value: Expr) -> Result<(), &'static str> {
    match name {
        "payload" => {
            node.payload = Some(value);
            Ok(())
        }
        "next" => {
            node.next = Some(value);
            Ok(())
        }
        "add" | "combine" => {
            node.add = Some(value);
            Ok(())
        }
        "mul" | "extend" => {
            node.mul = Some(value);
            Ok(())
        }
        "unit" | "one" => {
            node.unit = Some(value);
            Ok(())
        }
        "zero" => {
            node.zero = Some(value);
            Ok(())
        }
        _ => Err("unknown node field"),
    }
}

fn unescape_string(raw: &str) -> Result<String, &'static str> {
    // raw includes quotes: '...'
    if raw.len() < 2 {
        return Err("invalid string literal");
    }
    let inner = &raw[1..raw.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('\'') => out.push('\''),
                Some(other) => {
                    out.push(other);
                }
                None => return Err("invalid string escape"),
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

enum Postfix {
    Field(String),
    Method { name: String, args: Vec<Expr> },
    Index(Expr),
}

peg::parser! {
    grammar dim_parser() for [Spanned] {
        pub rule program(src: &str) -> Program =
            items:item(src)* {
                Program { items }
            }

        rule item(src: &str) -> Item =
            [Spanned { token: Token::Extern, .. }]
            [Spanned { token: Token::Type, .. }]
            name:ident(src) {
                Item::ExternType { name }
            }
            / [Spanned { token: Token::Data, .. }]
              name:ident(src)
              [Spanned { token: Token::Colon, .. }]
              ty:ty(src) {
                Item::Data { name, ty }
            }
            / name:ident(src)
              [Spanned { token: Token::Eq, .. }]
              value:expr(src) {
                Item::Assign { name, value }
            }

        rule ty(src: &str) -> Type =
            map_ty(src)
            / list_ty(src)
            / record_ty(src)
            / named_ty(src)

        rule named_ty(src: &str) -> Type =
            name:ident(src) { Type::Named(name) }

        rule list_ty(src: &str) -> Type =
            [Spanned { token: Token::LBracket, .. }]
            elem:ty(src)
            [Spanned { token: Token::RBracket, .. }] {
                Type::List(Box::new(elem))
            }

        rule map_ty(src: &str) -> Type =
            [Spanned { token: Token::LBrace, .. }]
            [Spanned { token: Token::LBracket, .. }]
            key:ty(src)
            [Spanned { token: Token::RBracket, .. }]
            [Spanned { token: Token::Colon, .. }]
            value:ty(src)
            [Spanned { token: Token::RBrace, .. }] {
                Type::Map {
                    key: Box::new(key),
                    value: Box::new(value),
                }
            }

        rule record_ty(src: &str) -> Type =
            [Spanned { token: Token::LBrace, .. }]
            fields:record_ty_fields(src)
            [Spanned { token: Token::RBrace, .. }] {
                Type::Record { fields }
            }

        rule record_ty_fields(src: &str) -> Vec<(String, Type)> =
            first:record_ty_field(src)
            rest:([Spanned { token: Token::Comma, .. }] f:record_ty_field(src) { f })* {
                let mut fields = vec![first];
                fields.extend(rest);
                fields
            }
            / { Vec::new() }

        rule record_ty_field(src: &str) -> (String, Type) =
            name:ident(src)
            [Spanned { token: Token::Colon, .. }]
            ty:ty(src) {
                (name, ty)
            }

        pub rule expr(src: &str) -> Expr = precedence! {
            x:(@) [Spanned { token: Token::EqEq, .. }] y:@ {
                c("__intrinsic_eq", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Ne, .. }] y:@ {
                c("__intrinsic_ne", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Le, .. }] y:@ {
                c("__intrinsic_le", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Ge, .. }] y:@ {
                c("__intrinsic_ge", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Lt, .. }] y:@ {
                c("__intrinsic_lt", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Gt, .. }] y:@ {
                c("__intrinsic_gt", vec![x, y])
            }
            --
            x:(@) [Spanned { token: Token::Plus, .. }] y:@ {
                c("__intrinsic_add", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Minus, .. }] y:@ {
                c("__intrinsic_sub", vec![x, y])
            }
            --
            x:(@) [Spanned { token: Token::Star, .. }] y:@ {
                c("__intrinsic_mul", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Slash, .. }] y:@ {
                c("__intrinsic_div", vec![x, y])
            }
            x:(@) [Spanned { token: Token::Percent, .. }] y:@ {
                c("__intrinsic_mod", vec![x, y])
            }
            --
            e:postfix(src) { e }
        }

        rule postfix(src: &str) -> Expr =
            atom:atom(src) ops:postfix_op(src)* {
                ops.into_iter().fold(atom, |base, op| match op {
                    Postfix::Field(field) => Expr::Field {
                        base: Box::new(base),
                        field,
                    },
                    Postfix::Method { name, args } => Expr::MethodCall {
                        base: Box::new(base),
                        method: name,
                        args,
                    },
                    Postfix::Index(index) => Expr::Index {
                        base: Box::new(base),
                        index: Box::new(index),
                    },
                })
            }

        rule postfix_op(src: &str) -> Postfix =
            [Spanned { token: Token::Dot, .. }] name:ident(src)
            [Spanned { token: Token::LParen, .. }]
            args:arg_list(src)
            [Spanned { token: Token::RParen, .. }] {
                Postfix::Method { name, args }
            }
            / [Spanned { token: Token::Dot, .. }] name:ident(src) {
                Postfix::Field(name)
            }
            / [Spanned { token: Token::LBracket, .. }]
              index:expr(src)
              [Spanned { token: Token::RBracket, .. }] {
                Postfix::Index(index)
            }

        rule atom(src: &str) -> Expr =
            [Spanned { token: Token::Minus, .. }]
            [Spanned { token: Token::Inf, .. }] {
                Expr::NegInf
            }
            / [Spanned { token: Token::Int, start, end }] {
                Expr::Lit(src[start..end].to_owned())
            }
            / [Spanned { token: Token::String, start, end }] {?
                unescape_string(&src[start..end]).map(Expr::Str)
            }
            / lambda(src)
            / dp_expr(src)
            / record_lit(src)
            / block(src)
            / call_or_name(src)
            / [Spanned { token: Token::LParen, .. }]
              e:expr(src)
              [Spanned { token: Token::RParen, .. }] {
                e
            }

        /// Non-empty record literal `{ field: expr, ... }`. Empty `{}` stays a block.
        rule record_lit(src: &str) -> Expr =
            [Spanned { token: Token::LBrace, .. }]
            first:record_lit_field(src)
            rest:([Spanned { token: Token::Comma, .. }] f:record_lit_field(src) { f })*
            [Spanned { token: Token::RBrace, .. }] {
                let mut fields = vec![first];
                fields.extend(rest);
                Expr::RecordLit(fields)
            }

        rule record_lit_field(src: &str) -> (String, Expr) =
            name:ident(src)
            [Spanned { token: Token::Colon, .. }]
            value:expr(src) {
                (name, value)
            }

        rule call_or_name(src: &str) -> Expr =
            name:callable_name(src)
            [Spanned { token: Token::LParen, .. }]
            args:arg_list(src)
            [Spanned { token: Token::RParen, .. }] {
                Expr::Call { name, args }
            }
            / name:callable_name(src) {
                Expr::Name(name)
            }

        rule callable_name(src: &str) -> String =
            ident(src)
            / [Spanned { token: Token::Node, .. }] { "node".to_owned() }

        rule arg_list(src: &str) -> Vec<Expr> =
            first:expr(src)
            rest:([Spanned { token: Token::Comma, .. }] e:expr(src) { e })* {
                let mut args = vec![first];
                args.extend(rest);
                args
            }
            / { Vec::new() }

        rule lambda(src: &str) -> Expr =
            [Spanned { token: Token::Pipe, .. }]
            params:lambda_params(src)
            [Spanned { token: Token::Pipe, .. }]
            body:expr(src) {
                Expr::Lambda {
                    params,
                    body: Box::new(body),
                }
            }

        rule lambda_params(src: &str) -> Vec<String> =
            first:ident(src)
            rest:([Spanned { token: Token::Comma, .. }] name:ident(src) { name })* {
                let mut params = vec![first];
                params.extend(rest);
                params
            }
            / { Vec::new() }

        rule block(src: &str) -> Expr =
            [Spanned { token: Token::LBrace, .. }]
            stmts:stmt(src)*
            [Spanned { token: Token::RBrace, .. }] {
                Expr::Block(stmts)
            }

        rule stmt(src: &str) -> Stmt =
            [Spanned { token: Token::For, .. }]
            var:ident(src)
            [Spanned { token: Token::In, .. }]
            iter:expr(src)
            [Spanned { token: Token::LBrace, .. }]
            body:stmt(src)*
            [Spanned { token: Token::RBrace, .. }] {
                Stmt::For { var, iter, body }
            }
            / [Spanned { token: Token::If, .. }]
              cond:expr(src)
              [Spanned { token: Token::LBrace, .. }]
              body:stmt(src)*
              [Spanned { token: Token::RBrace, .. }]
              else_body:else_body(src)? {
                Stmt::If { cond, body, else_body }
            }
            / [Spanned { token: Token::Yield, .. }]
              value:expr(src) {
                Stmt::Yield(value)
            }
            / e:expr(src) {
                Stmt::Expr(e)
            }

        rule else_body(src: &str) -> Vec<Stmt> =
            [Spanned { token: Token::Else, .. }]
            [Spanned { token: Token::LBrace, .. }]
            body:stmt(src)*
            [Spanned { token: Token::RBrace, .. }] {
                body
            }

        rule dp_expr(src: &str) -> Expr =
            [Spanned { token: Token::Dp, .. }]
            [Spanned { token: Token::LBrace, .. }]
            nodes:node_def(src)*
            [Spanned { token: Token::RBrace, .. }] {
                Expr::Dp(DpBlock { nodes })
            }

        rule node_def(src: &str) -> NodeDef =
            [Spanned { token: Token::Node, .. }]
            name:node_name_opt(src)
            [Spanned { token: Token::LBrace, .. }]
            fields:node_field(src)*
            [Spanned { token: Token::RBrace, .. }] {?
                let mut node = NodeDef {
                    name,
                    ..NodeDef::default()
                };
                for field in fields {
                    match field {
                        NodeField::Key(ty) => node.key = Some(ty),
                        NodeField::Bind { name, value } => {
                            apply_node_field(&mut node, &name, value)?;
                        }
                    }
                }
                Ok(node)
            }

        rule node_name_opt(src: &str) -> Option<String> =
            [Spanned { token: Token::LParen, .. }]
            [Spanned { token: Token::Ident, start, end }]
            [Spanned { token: Token::Eq, .. }]
            [Spanned { token: Token::String, start: s0, end: e0 }]
            [Spanned { token: Token::RParen, .. }] {?
                let attr = &src[start..end];
                if attr != "name" {
                    return Err("expected name = '...'");
                }
                unescape_string(&src[s0..e0]).map(Some)
            }
            / { None }

        rule node_field(src: &str) -> NodeField =
            name:ident(src)
            [Spanned { token: Token::Colon, .. }]
            ty:ty(src) {?
                if name == "key" {
                    Ok(NodeField::Key(ty))
                } else {
                    Err("expected key type field")
                }
            }
            / name:ident(src)
              [Spanned { token: Token::Eq, .. }]
              value:expr(src) {
                NodeField::Bind { name, value }
            }

        rule ident(src: &str) -> String =
            [Spanned { token: Token::Ident, start, end }] {
                src[start..end].to_owned()
            }
    }
}

enum NodeField {
    Key(Type),
    Bind { name: String, value: Expr },
}
