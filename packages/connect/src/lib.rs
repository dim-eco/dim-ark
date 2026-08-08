#![deny(clippy::all)]

mod frag;
mod value_convert;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dim_lang::{eval_paths_between, parse, set_data, Env, Program};
use frag::{parse_paths_between, BetweenArg};
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
        let (begin, end) = parse_paths_between(&src).map_err(Error::from_reason)?;
        Ok(Frag {
            begin,
            end,
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
    begin: BetweenArg,
    end: BetweenArg,
    session: Arc<Mutex<Session>>,
}

fn resolve_arg(arg: &BetweenArg, bindings: &HashMap<String, i64>) -> Result<i64> {
    match arg {
        BetweenArg::Lit(v) => Ok(*v),
        BetweenArg::Param(name) => bindings
            .get(name)
            .copied()
            .ok_or_else(|| Error::from_reason(format!("missing binding `{name}`"))),
    }
}

#[napi]
impl Frag {
    /// True when both endpoints are literals (no bindings needed).
    #[napi(js_name = "isComplete")]
    pub fn is_complete(&self) -> bool {
        matches!(self.begin, BetweenArg::Lit(_)) && matches!(self.end, BetweenArg::Lit(_))
    }

    #[napi]
    pub fn eval(&self, bindings: HashMap<String, i64>) -> Result<i64> {
        let begin = resolve_arg(&self.begin, &bindings)?;
        let end = resolve_arg(&self.end, &bindings)?;
        let session = self
            .session
            .lock()
            .map_err(|_| Error::from_reason("bucket session poisoned"))?;
        let program = session
            .program
            .as_ref()
            .ok_or_else(|| Error::from_reason("bucket not initialized"))?;
        eval_paths_between(program, &session.env, begin, end)
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}
