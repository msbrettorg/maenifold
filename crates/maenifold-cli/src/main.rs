fn main() {
    let code = maenifold_core::CliApp::run(&std::env::args().skip(1).collect::<Vec<_>>());
    std::process::exit(code);
}
