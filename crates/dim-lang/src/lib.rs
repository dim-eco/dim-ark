mod ast;
mod error;
mod interp;
mod parse;
mod token;
mod vm;

pub use ast::{DpBlock, Expr, Item, NodeDef, Program, Stmt, Type};
pub use error::Error;
pub use interp::{
    eval_dp_between, eval_dp_between_debug, eval_paths_between, eval_paths_between_debug,
    PathsBetweenNode, PathsBetweenResult,
};
pub use parse::{lex, parse, parse_expr};
pub use token::{Spanned, Token};
pub use vm::{eval, set_data, Env, Value};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn eval_src(src: &str) -> Result<String, Error> {
    eval(&parse_expr(src)?)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        eval_dp_between, eval_paths_between, eval_src, set_data, Env, Value,
    };
    use crate::ast::{
        c, field, index, lambda, lit, name, named_ty, DpBlock, Expr, Item, NodeDef, Program, Stmt,
        Type,
    };
    use crate::parse;

    fn expect_ok(src: &str, expected: &str) {
        assert_eq!(eval_src(src).unwrap(), expected);
    }

    #[test]
    fn add() {
        expect_ok("1+2", "3");
    }

    #[test]
    fn sub() {
        expect_ok("10-3", "7");
    }

    #[test]
    fn mul() {
        expect_ok("4*5", "20");
    }

    #[test]
    fn div() {
        expect_ok("20/4", "5");
    }

    #[test]
    fn rem() {
        expect_ok("10%3", "1");
    }

    #[test]
    fn precedence() {
        expect_ok("1+2*3", "7");
    }

    #[test]
    fn parens() {
        expect_ok("(1+2)*3", "9");
    }

    #[test]
    fn left_assoc() {
        expect_ok("10-3-2", "5");
    }

    #[test]
    fn nested() {
        expect_ok("(10+2)/(3+1)", "3");
    }

    #[test]
    fn whitespace() {
        expect_ok(" 1 + 2 * 3 ", "7");
    }

    #[test]
    fn compare_and_max() {
        expect_ok("1 < 2", "1");
        expect_ok("2 <= 2", "1");
        expect_ok("max(3, 5)", "5");
        expect_ok("-inf", "-inf");
    }

    const PATHS_SRC: &str = r#"
	  extern type Id
	  extern type Value
	  
	  data input: {values: {[Id]: Value}, edges: {[Id]: [Id]}}
	  
		paths = dp {
			node {
				key: Id
				payload = input.values[key]
				next = || {
					for to in input.edges[key] {
						yield node(to)	
					}	
				}
				
				// operations indeed are in the node definition
				
				combine = |a, b| a + b
				extend  = |a, b| a * b
				unit    = 1 
				zero = 0
			}
		}
	"#;

    const BACKPACK_SRC: &str = r#"
extern type Id
extern type Weight
extern type Value

data input: {first_id: Id, last_id: Id, capacity: Weight, items: {[Id]: {weight: Weight, value: Value}}}

backpack = dp {
    node (name = 'main') {
        key: {weight: Weight, id: Step}
        payload = 0
        next = || {
            if key.weight + input.items[key.id.current].weight <= input.capacity {
                yield node.take({
                    weight: key.weight + input.items[key.id.current].weight,
                    id: key.id
                })
            }
            if key.id.current == input.last_id {
                yield node.result()
            } else {
                yield node.main({weight: key.weight, id: key.id.incremented()})
            }
        }
        combine = |a, b| max(a, b)
        extend = |a, b| a + b
        unit = 0
        zero = 0
    }

    node (name = 'take') {
        key: {weight: Weight, id: Step}
        payload = input.items[key.id.current].value
        next = || {
            if key.id.current == input.last_id {
                yield node.result()
            } else {
                yield node.main({weight: key.weight, id: key.id.incremented()})
            }
        }
        combine = |a, b| max(a, b)
        extend = |a, b| a + b
        unit = 0
        zero = 0
    }

    node (name = 'result') {
        key: {}
        payload = 0
        next = || {}
        combine = |a, b| max(a, b)
        extend = |a, b| a + b
        unit = 0
        zero = 0
    }
}
"#;

    #[test]
    fn paths_new_model_ast() {
        let expected = Program {
            items: vec![
                Item::ExternType {
                    name: "Id".into(),
                },
                Item::ExternType {
                    name: "Value".into(),
                },
                Item::Data {
                    name: "input".into(),
                    ty: Type::Record {
                        fields: vec![
                            (
                                "values".into(),
                                Type::Map {
                                    key: Box::new(named_ty("Id")),
                                    value: Box::new(named_ty("Value")),
                                },
                            ),
                            (
                                "edges".into(),
                                Type::Map {
                                    key: Box::new(named_ty("Id")),
                                    value: Box::new(Type::List(Box::new(named_ty("Id")))),
                                },
                            ),
                        ],
                    },
                },
                Item::Assign {
                    name: "paths".into(),
                    value: Expr::Dp(DpBlock {
                        nodes: vec![NodeDef {
                            name: None,
                            key: Some(named_ty("Id")),
                            payload: Some(index(
                                field(name("input"), "values"),
                                name("key"),
                            )),
                            next: Some(lambda(
                                &[],
                                Expr::Block(vec![Stmt::For {
                                    var: "to".into(),
                                    iter: index(field(name("input"), "edges"), name("key")),
                                    body: vec![Stmt::Yield(c("node", vec![name("to")]))],
                                }]),
                            )),
                            add: Some(lambda(
                                &["a", "b"],
                                c("__intrinsic_add", vec![name("a"), name("b")]),
                            )),
                            mul: Some(lambda(
                                &["a", "b"],
                                c("__intrinsic_mul", vec![name("a"), name("b")]),
                            )),
                            unit: Some(lit("1")),
                            zero: Some(lit("0")),
                        }],
                    }),
                },
            ],
        };

        assert_eq!(parse(PATHS_SRC).unwrap(), expected);
    }

    #[test]
    fn paths_between_1_and_9() {
        let program = parse(PATHS_SRC).unwrap();
        let mut env = Env::new();

        let values: BTreeMap<i64, Value> = (1..=9).map(|i| (i, Value::Int(i))).collect();
        let edges: BTreeMap<i64, Value> = [
            (1, vec![2, 3]),
            (2, vec![4, 5]),
            (3, vec![5, 6]),
            (4, vec![7]),
            (5, vec![7, 9]),
            (6, vec![8]),
            (7, vec![9]),
            (8, vec![9]),
            (9, vec![]),
        ]
        .into_iter()
        .map(|(k, tos)| {
            (
                k,
                Value::List(tos.into_iter().map(Value::Int).collect()),
            )
        })
        .collect();

        let mut input_fields = BTreeMap::new();
        input_fields.insert("values".into(), Value::Map(values));
        input_fields.insert("edges".into(), Value::Map(edges));
        set_data(&mut env, "input", Value::Record(input_fields));

        assert_eq!(eval_paths_between(&program, &env, 1, 9).unwrap(), 3600);
    }

    #[test]
    fn backpack_parses() {
        parse(BACKPACK_SRC).unwrap();
    }

    #[test]
    fn backpack_between_capacity_14() {
        let program = parse(BACKPACK_SRC).unwrap();
        let mut env = Env::new();

        let mut items = BTreeMap::new();
        for (id, value, weight) in [(1, 3, 5), (2, 5, 10), (3, 4, 6), (4, 2, 5)] {
            let mut item = BTreeMap::new();
            item.insert("value".into(), Value::Int(value));
            item.insert("weight".into(), Value::Int(weight));
            items.insert(id, Value::Record(item));
        }

        let mut input = BTreeMap::new();
        input.insert("capacity".into(), Value::Int(14));
        input.insert("first_id".into(), Value::Int(1));
        input.insert("last_id".into(), Value::Int(4));
        input.insert("items".into(), Value::Map(items));
        set_data(&mut env, "input", Value::Record(input));

        let mut begin_key = BTreeMap::new();
        begin_key.insert("weight".into(), Value::Int(0));
        begin_key.insert("id".into(), Value::Step(1));
        let begin = Value::node_key("main", Value::Record(begin_key));
        let end = Value::node_key("result", Value::Record(BTreeMap::new()));

        assert_eq!(
            eval_dp_between(&program, &env, "backpack", begin, end).unwrap(),
            7
        );
    }
}
