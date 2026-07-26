<div align="center">

<img src="docs/assets/logo_light.png" alt="dimARK" width="160" />

# dimARK

**A wannabe DBMS for complex algorithmic problems.**

</div>

---

> ⚠️ **Very early days.** dimARK is at the very start of its road. APIs, syntax and storage
> format will change without warning. Nothing here is production-ready yet.

## What is dimARK?

dimARK is an experimental database management system built on a simple premise: the data of a
computational problem should live **inside** the database, and the database should be the thing
that computes over it.

The usual workflow is to pull a problem instance out of storage, solve it in application code, and
write the answer back. dimARK inverts that. You describe the problem once in **dim** — the DSL that
ships with dimARK — and the engine takes care of evaluation, storage layout, and recomputation when
the underlying data changes.

## Current status

Right now dimARK solves exactly one class of problems: **dynamic programming**.

That is the entire surface area today. Everything below this line is planned, not implemented.

## The dim DSL

Problems are described in `dim`, a small domain-specific language designed for this purpose. A dim
program declares the shape of the problem's data and the recurrence that defines its solution;
dimARK decides how to store and evaluate it.

<!-- TODO: replace with a real, working example -->
```dim
dp (name = lcs) {
    input = {s1: string, s2: string}
    node {
        key = {s1_index: int, s2_index: int}
        payload = |{key: {s1_index, s2_index}, input: {s1, s2}}| {
            if s1_index >= s1.len || s2_index >= s2.len {
                return 0;
            }
            if s1[s1_index] == s2[s2_index] {
                return 1;
            }
            return 0;
        }
        add = |a, b| max(a, b)
        mul = |a, b| a + b
        next = |{key: {s1_index, s2_index}, input: {s1, s2}}| {
            if s1_index >= s1.len || s2_index >= s2.len {
                return [];
            }
            if s1[s1_index] == s2[s2_index] {
                return [
                    node({
                        s1_index: s1_index + 1,
                        s2_index: s2_index + 1,
                    })
                ];
            }
            return [
                node({
                    s1_index: s1_index + 1,
                    s2_index: s2_index,
                }),
                node({
                    s1_index: s1_index,
                    s2_index: s2_index + 1,
                }),
            ];
        }
    }
}
```

## Roadmap

The goal is for dimARK to eventually earn the "DBMS" in its name — not just to compute, but to
store durably and behave correctly under failure and concurrency.

## Installation

<!-- TODO: confirm this still matches the current release -->
The JavaScript connector is published on npm:

```bash
npm install @dim-ark/connect
```

## Contributing

TODO

## License

AGPL-3.0-only. See [LICENSE](LICENSE).