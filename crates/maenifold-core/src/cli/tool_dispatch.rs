use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use serde_json::Value;
use std::future::Future;
use std::time::Duration;

const TOOL_DISPATCH_TIMEOUT: Duration = Duration::from_secs(30);
const TOOL_DISPATCH_TIMEOUT_MESSAGE: &str = "ERROR: Tool dispatch exceeded 30s timeout — the CLI may be blocked on a daemon-only path. If this is SequentialThinking with a conclusion, try invoking via 'curl http://localhost:8080/mcp' GatewayExecute instead.";

pub(super) fn invoke_tool_with_timeout(
    tool_name: &str,
    payload: Value,
    output_format: OutputFormat,
) -> Result<String> {
    let tool_name = tool_name.to_string();
    let future = async move {
        tokio::task::spawn_blocking(move || {
            ToolRegistry::global().try_invoke(&tool_name, &payload, output_format)
        })
        .await
        .map_err(|error| MaenifoldError::Message(format!("Tool dispatch thread failed: {error}")))?
    };

    block_on_tool_dispatch_with_timeout(future, TOOL_DISPATCH_TIMEOUT)
}

fn block_on_tool_dispatch_with_timeout<F>(future: F, timeout: Duration) -> Result<String>
where
    F: Future<Output = Result<String>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            MaenifoldError::Message(format!(
                "Failed to build CLI tool dispatch runtime: {error}"
            ))
        })?;

    let result = runtime.block_on(async {
        tokio::time::timeout(timeout, future)
            .await
            .map_err(|_| MaenifoldError::Message(TOOL_DISPATCH_TIMEOUT_MESSAGE.to_string()))?
    });
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}

#[cfg(test)]
mod tests {
    use super::block_on_tool_dispatch_with_timeout;
    use crate::error::Result;
    use std::future;
    use std::time::{Duration, Instant};

    #[test]
    fn tool_dispatch_timeout_returns_bounded_error() {
        let started = Instant::now();
        let result = block_on_tool_dispatch_with_timeout(
            future::pending::<Result<String>>(),
            Duration::from_millis(10),
        );

        assert!(started.elapsed() < Duration::from_secs(5));
        let message = result
            .expect_err("pending dispatch must time out")
            .to_string();
        assert!(message.contains("Tool dispatch exceeded 30s timeout"));
        assert!(message.contains("GatewayExecute"));
    }
}
