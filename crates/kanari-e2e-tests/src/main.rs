fn main() {
    eprintln!("kanari-e2e-tests: run with `cargo test -p kanari-e2e-tests`");
    eprintln!("live-node tests need KANARI_E2E_RPC_URL, e.g.:");
    eprintln!("  $env:KANARI_E2E_RPC_URL='http://127.0.0.1:6767'; cargo test -p kanari-e2e-tests");
}
