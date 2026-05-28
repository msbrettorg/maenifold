use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use serde_json::Value;
use std::env;
use std::future::Future;
use std::time::Duration;

const DEFAULT_TOOL_DISPATCH_TIMEOUT_SECS: u64 = 30;

fn tool_dispatch_timeout() -> Duration {
    Duration::from_secs(
        env::var("MAENIFOLD_TOOL_DISPATCH_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_TOOL_DISPATCH_TIMEOUT_SECS),
    )
}

fn tool_dispatch_timeout_message(timeout: Duration) -> String {
    format!(
        "ERROR: Tool dispatch exceeded {}s timeout — the CLI may be blocked on a daemon-only path. If this is SequentialThinking with a conclusion, try invoking via 'curl http://localhost:8080/mcp' GatewayExecute instead.",
        timeout.as_secs()
    )
}

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

    block_on_tool_dispatch_with_timeout(future, tool_dispatch_timeout())
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
            .map_err(|_| MaenifoldError::Message(tool_dispatch_timeout_message(timeout)))?
    });
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_TOOL_DISPATCH_TIMEOUT_SECS, block_on_tool_dispatch_with_timeout,
        tool_dispatch_timeout, tool_dispatch_timeout_message,
    };
    use crate::error::Result;
    use serial_test::serial;
    use std::env;
    use std::future;
    use std::time::{Duration, Instant};

    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let original = env::var(key).ok();
            unsafe {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(value) => env::set_var(self.key, value),
                    None => env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn tool_dispatch_resolves_value_within_timeout() {
        let result = block_on_tool_dispatch_with_timeout(
            async { Ok("expected-value".to_string()) },
            Duration::from_secs(5),
        );
        assert_eq!(result.expect("future resolves"), "expected-value");
    }

    #[test]
    #[serial]
    fn tool_dispatch_timeout_defaults_to_30_seconds_when_env_unset() {
        let _guard = EnvVarGuard::set("MAENIFOLD_TOOL_DISPATCH_TIMEOUT_SECS", None);

        assert_eq!(
            tool_dispatch_timeout(),
            Duration::from_secs(DEFAULT_TOOL_DISPATCH_TIMEOUT_SECS)
        );
    }

    #[test]
    #[serial]
    fn tool_dispatch_timeout_message_reflects_configured_timeout() {
        let _guard = EnvVarGuard::set("MAENIFOLD_TOOL_DISPATCH_TIMEOUT_SECS", Some("120"));

        let timeout = tool_dispatch_timeout();
        let message = tool_dispatch_timeout_message(timeout);

        assert_eq!(timeout, Duration::from_secs(120));
        assert!(message.contains("Tool dispatch exceeded 120s timeout"));
        assert!(message.contains("GatewayExecute"));
    }

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
        assert!(message.contains("Tool dispatch exceeded"));
        assert!(message.contains("GatewayExecute"));
    }
}
