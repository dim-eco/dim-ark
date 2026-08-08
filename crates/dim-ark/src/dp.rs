use std::collections::{HashMap, HashSet, VecDeque};
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

/// Forward topological DP for paths from `begin` to `end`.
///
/// For each node `v` on some `begin → … → end` path:
/// - incoming at `begin` is `unit`; elsewhere `add` of predecessors' DP values (or `zero`)
/// - `dp[v] = mul(payload[v], incoming[v])`
///
/// Returns `dp[end]`, or `zero` if `end` is unreachable from `begin`.
pub fn solve_between<K, V>(
    edges: &HashMap<K, Vec<K>>,
    payload: &HashMap<K, V>,
    begin: &K,
    end: &K,
    mut add: impl FnMut(V, V) -> V,
    mut mul: impl FnMut(V, V) -> V,
    unit: V,
    zero: V,
) -> V
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    let mut reverse: HashMap<K, Vec<K>> = HashMap::new();
    for (from, tos) in edges {
        for to in tos {
            reverse.entry(to.clone()).or_default().push(from.clone());
        }
    }

    let from_begin = reachable(begin, edges);
    let to_end = reachable(end, &reverse);
    let nodes: HashSet<K> = from_begin.intersection(&to_end).cloned().collect();

    if !nodes.contains(end) {
        return zero;
    }

    let mut indegree: HashMap<K, usize> = HashMap::new();
    let mut forward: HashMap<K, Vec<K>> = HashMap::new();
    for n in &nodes {
        indegree.entry(n.clone()).or_insert(0);
    }
    for from in &nodes {
        for to in edges.get(from).into_iter().flatten() {
            if nodes.contains(to) {
                forward.entry(from.clone()).or_default().push(to.clone());
                *indegree.entry(to.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut queue: VecDeque<K> = indegree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(k, _)| k.clone())
        .collect();
    let mut order = Vec::with_capacity(nodes.len());
    let mut indegree = indegree;
    while let Some(n) = queue.pop_front() {
        order.push(n.clone());
        for to in forward.get(&n).into_iter().flatten() {
            let d = indegree.get_mut(to).expect("indegree for subgraph node");
            *d -= 1;
            if *d == 0 {
                queue.push_back(to.clone());
            }
        }
    }

    let mut dp: HashMap<K, V> = HashMap::new();
    for v in &order {
        let incoming = if v == begin {
            unit.clone()
        } else {
            reverse
                .get(v)
                .into_iter()
                .flatten()
                .filter(|p| nodes.contains(p))
                .filter_map(|p| dp.get(p).cloned())
                .reduce(|a, b| add(a, b))
                .unwrap_or_else(|| zero.clone())
        };

        let p = payload
            .get(v)
            .cloned()
            .unwrap_or_else(|| zero.clone());
        dp.insert(v.clone(), mul(p, incoming));
    }

    dp.get(end).cloned().unwrap_or(zero)
}

fn reachable<K: Clone + Eq + Hash>(start: &K, edges: &HashMap<K, Vec<K>>) -> HashSet<K> {
    let mut seen = HashSet::new();
    let mut stack = vec![start.clone()];
    while let Some(n) = stack.pop() {
        if !seen.insert(n.clone()) {
            continue;
        }
        if let Some(nexts) = edges.get(&n) {
            stack.extend(nexts.iter().cloned());
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::solve_between;
    use std::collections::HashMap;

    #[test]
    fn paths_between_1_and_9() {
        let payload: HashMap<i64, i64> = (1..=9).map(|i| (i, i)).collect();
        let edges: HashMap<i64, Vec<i64>> = [
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
        .collect();

        let result = solve_between(
            &edges,
            &payload,
            &1,
            &9,
            |a, b| a + b,
            |a, b| a * b,
            1,
            0,
        );
        assert_eq!(result, 3600);
    }
}
