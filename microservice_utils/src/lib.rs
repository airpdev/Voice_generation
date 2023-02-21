use std::{env, ffi::OsStr};

pub mod server;
pub mod jwt;
pub mod open_api;

pub use bhuman_micros;

#[macro_use]
extern crate lazy_static;

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}

lazy_static! {
    static ref USER_SERVICE_URL: String = ensure_var("USER_SERVICE_URL").unwrap();
    static ref AUTH_SERVICE_URL: String = ensure_var("AUTH_SERVICE_URL").unwrap();
    static ref WORKSPACE_SERVICE_URL: String = ensure_var("WORKSPACE_SERVICE_URL").unwrap();
    static ref AISTUDIO_URL: String = ensure_var("AISTUDIO_URL").unwrap();
    static ref FILE_SERVICE_URL: String = ensure_var("FILE_SERVICE_URL").unwrap();        
}