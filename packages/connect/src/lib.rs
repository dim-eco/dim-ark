#![deny(clippy::all)]

mod frag;
mod value_convert;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dim_lang::{
    eval_dp_between, eval_dp_between_debug, parse, set_data, Env, Program,
};
use frag::{
    endpoint_is_complete, parse_between_frag, resolve_endpoint, BetweenFrag,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use value_convert::json_to_value;

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

struct Session {
    program: Option<Program>,
    env: Env,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            program: None,
            env: Env::new(),
        }
    }
}

#[napi]
pub struct Bucket {
    #[allow(dead_code)]
    name: String,
    session: Arc<Mutex<Session>>,
}

#[napi(object)]
pub struct InitializeOptions {
    pub model: String,
    #[napi(js_name = "externalTypes")]
    pub external_types: Option<HashMap<String, String>>,
}

#[napi(object)]
pub struct BetweenNodeDebug {
    pub id: String,
    pub value: i64,
    pub sum: i64,
    pub dp: i64,
}

#[napi(object)]
pub struct BetweenEdgeDebug {
    pub from: String,
    pub to: String,
}

/// Debug payload from `*.between`: scalar result plus subgraph DP values.
#[napi(object)]
pub struct BetweenDebug {
    pub result: i64,
    pub nodes: Vec<BetweenNodeDebug>,
    pub edges: Vec<BetweenEdgeDebug>,
}

#[napi]
pub fn bucket(name: String) -> Bucket {
    Bucket {
        name,
        session: Arc::new(Mutex::new(Session::default())),
    }
}

#[napi]
impl Bucket {
    #[napi]
    pub fn initialize(&self, opts: InitializeOptions) -> Result<()> {
        let _ = opts.external_types;
        let program = parse(&opts.model).map_err(|e| Error::from_reason(e.to_string()))?;
        let mut session = self
            .session
            .lock()
            .map_err(|_| Error::from_reason("bucket session poisoned"))?;
        session.program = Some(program);
        session.env = Env::new();
        Ok(())
    }

    #[napi(js_name = "setData")]
    pub fn set_data(&self, name: String, value: serde_json::Value) -> Result<()> {
        let converted = json_to_value(value)?;
        let mut session = self
            .session
            .lock()
            .map_err(|_| Error::from_reason("bucket session poisoned"))?;
        set_data(&mut session.env, &name, converted);
        Ok(())
    }

    #[napi(js_name = "snapshotEnv")]
    pub fn snapshot_env(&self) -> Result<EnvSnapshot> {
        let session = self
            .session
            .lock()
            .map_err(|_| Error::from_reason("bucket session poisoned"))?;
        Ok(EnvSnapshot {
            env: session.env.clone(),
        })
    }

    #[napi(js_name = "restoreEnv")]
    pub fn restore_env(&self, snapshot: &EnvSnapshot) -> Result<()> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| Error::from_reason("bucket session poisoned"))?;
        session.env = snapshot.env.clone();
        Ok(())
    }

    #[napi(js_name = "prepareFrag")]
    pub fn prepare_frag(&self, src: String) -> Result<Frag> {
        let between = parse_between_frag(&src).map_err(Error::from_reason)?;
        Ok(Frag {
            between,
            session: Arc::clone(&self.session),
        })
    }
}

#[napi]
pub struct EnvSnapshot {
    env: Env,
}

#[napi]
pub struct Frag {
    between: BetweenFrag,
    session: Arc<Mutex<Session>>,
}

#[napi]
impl Frag {
    /// True when both endpoints are fully literal (no bindings needed).
    #[napi(js_name = "isComplete")]
    pub fn is_complete(&self) -> bool {
        endpoint_is_complete(&self.between.begin) && endpoint_is_complete(&self.between.end)
    }

    #[napi]
    pub fn eval(&self, bindings: HashMap<String, i64>) -> Result<i64> {
        let session = self
            .session
            .lock()
            .map_err(|_| Error::from_reason("bucket session poisoned"))?;
        let begin = resolve_endpoint(&self.between.begin, &bindings, &session.env)
            .map_err(Error::from_reason)?;
        let end = resolve_endpoint(&self.between.end, &bindings, &session.env)
            .map_err(Error::from_reason)?;
        let program = session
            .program
            .as_ref()
            .ok_or_else(|| Error::from_reason("bucket not initialized"))?;
        eval_dp_between(program, &session.env, &self.between.dp_name, begin, end)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Evaluate `between` and return the scalar plus per-node DP debug info.
    #[napi(js_name = "evalDebug")]
    pub fn eval_debug(&self, bindings: HashMap<String, i64>) -> Result<BetweenDebug> {
        let session = self
            .session
            .lock()
            .map_err(|_| Error::from_reason("bucket session poisoned"))?;
        let begin = resolve_endpoint(&self.between.begin, &bindings, &session.env)
            .map_err(Error::from_reason)?;
        let end = resolve_endpoint(&self.between.end, &bindings, &session.env)
            .map_err(Error::from_reason)?;
        let program = session
            .program
            .as_ref()
            .ok_or_else(|| Error::from_reason("bucket not initialized"))?;
        let traced =
            eval_dp_between_debug(program, &session.env, &self.between.dp_name, begin, end)
                .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(BetweenDebug {
            result: traced.result,
            nodes: traced
                .nodes
                .into_iter()
                .map(|n| BetweenNodeDebug {
                    id: n.id,
                    value: n.value,
                    sum: n.sum,
                    dp: n.dp,
                })
                .collect(),
            edges: traced
                .edges
                .into_iter()
                .map(|(from, to)| BetweenEdgeDebug { from, to })
                .collect(),
        })
    }
}
