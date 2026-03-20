// T-29.2: RTM FR-35 — dispatch tray-enabled modes (--http) to
// run_with_tray so the native event loop runs on the main thread and
// the tokio/axum server runs on a background std::thread.
// All other modes (--mcp, --tool, --help, errors) stay headless.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let code = match maenifold_core::CliApp::classify(&args) {
        maenifold_core::CliAction::Http(port) => {
            maenifold_core::tray::run_with_tray(maenifold_core::tray::ServerMode::Http(port))
        }
        maenifold_core::CliAction::Headless => maenifold_core::CliApp::run(&args),
    };

    std::process::exit(code);
}
