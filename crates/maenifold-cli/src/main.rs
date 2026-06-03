// T-29.2: RTM FR-35 — dispatch tray-enabled modes (--http) to
// run_with_tray so the native event loop runs on the main thread and
// the tokio/axum server runs on a background std::thread.
// All other modes (--mcp, --tool, --help, errors) stay headless.
fn main() {
    let config = maenifold_core::Config::current();
    let _ = config.ensure_directories();
    // Tell the Jockey adapter where to find maenifold.json for agent resolution.
    jockey::acp::set_config_root(config.maenifold_root.clone());
    // T-92.2: hold guard in main scope so the non-blocking writer flushes on drop.
    let _guard = maenifold_core::logging::init_tracing(&config);

    let args: Vec<String> = std::env::args().skip(1).collect();

    let code = match maenifold_core::CliApp::classify(&args) {
        maenifold_core::CliAction::Http { port, .. } => {
            maenifold_core::tray::run_with_tray(maenifold_core::tray::ServerMode::Http(port))
        }
        maenifold_core::CliAction::Mcp { .. } => maenifold_core::CliApp::run(&args),
        maenifold_core::CliAction::Headless => maenifold_core::CliApp::run(&args),
    };

    // Explicit drop ensures buffered events are flushed before exit.
    drop(_guard);
    std::process::exit(code);
}
