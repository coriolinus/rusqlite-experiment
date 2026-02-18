use std::{convert::Infallible, fmt::Display};

use anyhow::anyhow;
use serde_json::json;
use wasm_bindgen::prelude::*;

/// A convenience wrapper for results which defaults to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A Js-compatible error type
#[derive(Debug, derive_more::Display, derive_more::From)]
pub struct Error(anyhow::Error);

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<Error> for JsValue {
    fn from(value: Error) -> Self {
        // construct the basic error
        let mut json_value = json!({
            "msg": value.to_string(),
        });

        // fill in the source errors chain
        let mut err: &dyn std::error::Error = &value;
        let mut json_err = &mut json_value;
        while let Some(inner_err) = err.source() {
            json_err.as_object_mut().unwrap().insert(
                "source".into(),
                json!({
                    "msg": inner_err.to_string(),
                }),
            );

            err = inner_err;
            json_err = json_err.as_object_mut().unwrap().get_mut("source").unwrap();
        }

        // convert to JsValue
        serde_wasm_bindgen::to_value(&json_value)
            .expect("converting a simple object tree to js works")
    }
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        let value = serde_wasm_bindgen::from_value::<serde_json::Value>(value.clone())
            .ok()
            .or_else(|| value.as_string().map(Into::into))
            .and_then(|value| serde_json::to_string(&value).ok())
            .unwrap_or("js error".into());
        Self(anyhow!(value))
    }
}

impl From<idb::Error> for Error {
    fn from(value: idb::Error) -> Self {
        Self(anyhow!("{value}"))
    }
}

impl From<serde_wasm_bindgen::Error> for Error {
    fn from(value: serde_wasm_bindgen::Error) -> Self {
        Self(anyhow!("{value}"))
    }
}

/// Blatant ripoff of what anyhow does so that we can use it as conveniently
pub trait Context<T, E> {
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: 'static + Display + Send + Sync;
}

impl<T> Context<T, Infallible> for Option<T> {
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: 'static + Display + Send + Sync,
    {
        <Option<T> as anyhow::Context<T, Infallible>>::context(self, context).map_err(Error)
    }
}

impl<T, E> Context<T, E> for Result<T, E>
where
    Result<T, E>: anyhow::Context<T, E>,
{
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: 'static + Display + Send + Sync,
    {
        <Result<T, E> as anyhow::Context<T, E>>::context(self, context).map_err(Error)
    }
}
