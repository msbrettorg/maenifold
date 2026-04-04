use serde::{Serialize, de::DeserializeOwned};

use super::types::{AgentError, Result};

pub(super) fn bridge<T, U>(value: T) -> Result<U>
where
    T: Serialize,
    U: DeserializeOwned,
{
    serde_json::from_value(serde_json::to_value(value).map_err(json_error)?).map_err(json_error)
}

pub(super) fn json_error(error: serde_json::Error) -> AgentError {
    AgentError::Acp(format!("ACP type bridge failed: {error}"))
}
