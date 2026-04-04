use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::workflow::pipeline::{
    execute_pipeline, finalize_failed_pipeline_execution, persist_final_pipeline_status,
};
use maenifold_core::{Config, MaenifoldError, Result, logging};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let config = Config::current();
    if let Err(error) = config.ensure_directories() {
        eprintln!("ERROR: {error}");
        std::process::exit(1);
    }
    let guard = logging::init_tracing(&config);

    let code = match run(config).await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("ERROR: {error}");
            1
        }
    };

    drop(guard);
    std::process::exit(code);
}

async fn run(config: Config) -> Result<()> {
    let (session_id, prompt) = parse_args()?;
    let context = ToolContext::new(config, OutputFormat::Json);
    let cancel = CancellationToken::new();

    let signal_cancel = cancel.clone();
    let signal_session_id = session_id.clone();
    let signal_task = tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::warn!(
                    session_id = %signal_session_id,
                    "[pipeline-driver] received ctrl-c; cancelling pipeline"
                );
                signal_cancel.cancel();
            }
            Err(error) => {
                tracing::error!(
                    session_id = %signal_session_id,
                    error = %error,
                    "[pipeline-driver] failed to listen for ctrl-c"
                );
            }
        }
    });

    let execution_result = execute_pipeline(&context, &session_id, &prompt, cancel).await;
    signal_task.abort();
    let _ = signal_task.await;

    match execution_result {
        Ok(execution_result) => {
            persist_final_pipeline_status(&context, &session_id, &execution_result.status)?;
            Ok(())
        }
        Err(error) => {
            finalize_failed_pipeline_execution(&context, &session_id);
            Err(error)
        }
    }
}

fn parse_args() -> Result<(String, String)> {
    let mut args = std::env::args().skip(1);
    let session_id = args.next().ok_or_else(|| {
        MaenifoldError::Message(
            "usage: maenifold-pipeline-driver <session-id> [prompt ...]".to_string(),
        )
    })?;
    let prompt = args.collect::<Vec<_>>().join(" ");
    Ok((session_id, prompt))
}
