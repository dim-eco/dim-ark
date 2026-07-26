#![deny(clippy::all)]

use napi_derive::napi;

#[napi]
pub fn version() -> String {
    dim_ark::version().to_string()
}

#[napi(js_name = "test_subset_sum")]
pub fn test_subset_sum(a: Vec<i64>, t: i64) -> bool {
    dim_ark::test_subset_sum(a, t)
}

#[napi(js_name = "test_lcs")]
pub fn test_lcs(s1: String, s2: String) -> i64 {
    dim_ark::test_lcs(s1, s2)
}
