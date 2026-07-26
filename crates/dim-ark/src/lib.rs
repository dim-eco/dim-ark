pub mod dp;
pub mod lcs;
pub mod subset_sum;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn test_subset_sum(a: Vec<i64>, t: i64) -> bool {
    subset_sum::subset_sum(subset_sum::Input {
        a: a.into_boxed_slice(),
        t,
    })
}

pub fn test_lcs(s1: String, s2: String) -> i64 {
    lcs::lcs(lcs::Input { s1, s2 })
}
