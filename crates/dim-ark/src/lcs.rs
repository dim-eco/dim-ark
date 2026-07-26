use crate::dp::{self, Dp, Effect};

pub struct Input {
    pub s1: String,
    pub s2: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    s1_index: usize,
    s2_index: usize,
}

struct Lcs;

impl Dp for Lcs {
    type Input = Input;
    type Key = Key;
    type Value = i64;

    fn start(_input: &Self::Input) -> Self::Key {
        Key {
            s1_index: 0,
            s2_index: 0,
        }
    }

    fn effect(key: &Self::Key, input: &Self::Input) -> Effect<Self::Value> {
        let s1 = input.s1.as_bytes();
        let s2 = input.s2.as_bytes();

        if key.s1_index >= s1.len() || key.s2_index >= s2.len() {
            return Effect::Value(0);
        }

        if s1[key.s1_index] == s2[key.s2_index] {
            Effect::Value(1)
        } else {
            Effect::Value(0)
        }
    }

    fn next(key: &Self::Key, input: &Self::Input) -> Vec<Self::Key> {
        let s1 = input.s1.as_bytes();
        let s2 = input.s2.as_bytes();

        if key.s1_index >= s1.len() || key.s2_index >= s2.len() {
            return Vec::new();
        }

        if s1[key.s1_index] == s2[key.s2_index] {
            return vec![Key {
                s1_index: key.s1_index + 1,
                s2_index: key.s2_index + 1,
            }];
        }

        vec![
            Key {
                s1_index: key.s1_index + 1,
                s2_index: key.s2_index,
            },
            Key {
                s1_index: key.s1_index,
                s2_index: key.s2_index + 1,
            },
        ]
    }

    fn add(a: Self::Value, b: Self::Value) -> Self::Value {
        a.max(b)
    }

    fn mul(a: Self::Value, b: Self::Value) -> Self::Value {
        a + b
    }

    fn zero() -> Self::Value {
        0
    }
}

pub fn lcs(input: Input) -> i64 {
    dp::solve::<Lcs>(&input)
}
