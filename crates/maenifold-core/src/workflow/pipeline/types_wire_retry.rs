use super::PipelineNodeWire;
use crate::workflow::pipeline::RetryPolicy;

pub(super) fn parse_retry_policy(
    wire: &PipelineNodeWire,
    node_id: &str,
) -> Result<Option<RetryPolicy>, String> {
    let has_retry_field = wire.retry_max_attempts.is_some()
        || wire.retry_backoff_min_ms.is_some()
        || wire.retry_backoff_max_ms.is_some()
        || wire.retry_jitter.is_some();

    if !has_retry_field {
        return Ok(None);
    }

    let max_attempts = wire.retry_max_attempts.ok_or_else(|| {
        format!("Node '{node_id}': retryMaxAttempts is required when retry fields are present")
    })?;
    let backoff_min_ms = wire.retry_backoff_min_ms.ok_or_else(|| {
        format!("Node '{node_id}': retryBackoffMinMs is required when retry fields are present")
    })?;
    let backoff_max_ms = wire.retry_backoff_max_ms.ok_or_else(|| {
        format!("Node '{node_id}': retryBackoffMaxMs is required when retry fields are present")
    })?;
    let jitter = wire.retry_jitter.ok_or_else(|| {
        format!("Node '{node_id}': retryJitter is required when retry fields are present")
    })?;

    if backoff_min_ms > backoff_max_ms {
        return Err(format!(
            "Node '{node_id}': retryBackoffMinMs ({backoff_min_ms}) must be <= retryBackoffMaxMs ({backoff_max_ms})"
        ));
    }

    Ok(Some(RetryPolicy {
        max_attempts,
        backoff_min_ms,
        backoff_max_ms,
        jitter,
    }))
}
