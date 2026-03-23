use std::process::ExitCode;

use maenifold_gate::cli::run_from;

fn main() -> ExitCode {
    let code = run_from(std::env::args_os());
    ExitCode::from(code as u8)
}
