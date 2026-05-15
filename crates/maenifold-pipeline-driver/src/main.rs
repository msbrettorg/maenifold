use maenifold_core::{Config, logging};
use tracing::error;

#[tokio::main]
async fn main() {
    let config = Config::current();
    if let Err(error) = config.ensure_directories() {
        eprintln!("failed to ensure pipeline driver directories: {error}");
        std::process::exit(1);
    }
    let guard = logging::init_tracing(&config);

    let code = match maenifold_pipeline_driver::run(config).await {
        Ok(()) => 0,
        Err(error) => {
            error!(error = %error, "pipeline driver execution failed");
            1
        }
    };

    drop(guard);
    std::process::exit(code);
}
