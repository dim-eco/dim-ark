use crate::dp::{self, Dp, Effect};

pub struct Input {
    pub a: Box<[i64]>,
    pub t: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    value: i64,
    index: usize,
}

struct SubsetSum;

impl Dp for SubsetSum {
    type Input = Input;
    type Key = Key;
    type Value = bool;

    fn start(_input: &Self::Input) -> Self::Key {
        Key {
            value: 0,
            index: 0,
        }
    }

    fn effect(key: &Self::Key, input: &Self::Input) -> Effect<Self::Value> {
        if key.value > input.t {
            return Effect::Drop;
        }

        if key.value == input.t {
            return Effect::Halt(true);
        }

        Effect::Value(false)
    }

    fn next(key: &Self::Key, input: &Self::Input) -> Vec<Self::Key> {
        if key.index >= input.a.len() {
            return Vec::new();
        }

        let item = input.a[key.index];
        vec![
            Key {
                value: key.value + item,
                index: key.index + 1,
            },
            Key {
                value: key.value,
                index: key.index + 1,
            },
        ]
    }

    fn add(a: Self::Value, b: Self::Value) -> Self::Value {
        a || b
    }

    fn mul(a: Self::Value, b: Self::Value) -> Self::Value {
        a || b
    }

    fn zero() -> Self::Value {
        false
    }
}

pub fn subset_sum(input: Input) -> bool {
    dp::solve::<SubsetSum>(&input)
}
