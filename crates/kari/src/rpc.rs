use jsonrpc_core::IoHandler;
use jsonrpc_http_server::{ServerBuilder, DomainsValidation, AccessControlAllowOrigin};
use serde_json::json;

pub fn start_rpc_server(rpc_port: u16) {
    let mut io = IoHandler::new();

    io.add_method("ping", |_| async {
        Ok(json!("pong"))
    });

    let server = ServerBuilder::new(io)
        .threads(3)
        .cors(DomainsValidation::AllowOnly(vec![AccessControlAllowOrigin::Any]))
        .start_http(&format!("0.0.0.0:{}", rpc_port).parse().unwrap())
        .expect("Unable to start RPC server");

    server.wait();
}