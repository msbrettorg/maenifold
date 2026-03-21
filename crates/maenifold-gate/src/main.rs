use std::process::ExitCode;

fn main() -> ExitCode {
    let code = maenifold_gate::run_from(std::env::args_os());
    ExitCode::from(code as u8)
}
