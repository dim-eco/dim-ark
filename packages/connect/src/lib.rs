#![deny(clippy::all)]

use napi_derive::napi;

#[napi]
pub fn version() -> String {
    dim_ark::version().to_string()
}
