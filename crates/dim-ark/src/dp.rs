use std::collections::HashMap;
use std::hash::Hash;

pub enum Effect<V> {
    Drop,
    Halt(V),
    Value(V),
}

pub trait Dp {
    type Input;
    type Key: Clone + Eq + Hash;
    type Value: Clone;

    fn start(input: &Self::Input) -> Self::Key;
    fn effect(key: &Self::Key, input: &Self::Input) -> Effect<Self::Value>;
    fn next(key: &Self::Key, input: &Self::Input) -> Vec<Self::Key>;

    fn add(a: Self::Value, b: Self::Value) -> Self::Value;
    fn mul(a: Self::Value, b: Self::Value) -> Self::Value;
    fn zero() -> Self::Value;
}

pub fn solve<P: Dp>(input: &P::Input) -> P::Value {
    let mut memo = HashMap::new();
    let mut halted = None;
    eval::<P>(&P::start(input), input, &mut memo, &mut halted)
}

fn eval<P: Dp>(
    key: &P::Key,
    input: &P::Input,
    memo: &mut HashMap<P::Key, P::Value>,
    halted: &mut Option<P::Value>,
) -> P::Value {
    if let Some(v) = halted.clone() {
        return v;
    }

    if let Some(cached) = memo.get(key) {
        return cached.clone();
    }

    let result = match P::effect(key, input) {
        Effect::Drop => P::zero(),
        Effect::Halt(v) => {
            *halted = Some(v.clone());
            v
        }
        Effect::Value(p) => {
            let combined = P::next(key, input)
                .into_iter()
                .map(|child| eval::<P>(&child, input, memo, halted))
                .reduce(P::add)
                .unwrap_or_else(P::zero);

            if let Some(v) = halted.clone() {
                v
            } else {
                P::mul(p, combined)
            }
        }
    };

    memo.insert(key.clone(), result.clone());
    result
}
