fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let envelope = wificalling_location_gateway::roundtrip_synthetic_probe()?;
    if envelope.payload != [1, 2, 3, 4, 5] {
        return Err("synthetic protobuf roundtrip failed".into());
    }

    let tls_stack = wificalling_location_gateway::build_tls_stack()?;
    if tls_stack.report.server_alpn_protocols != 1 || tls_stack.report.client_alpn_protocols != 1 {
        return Err("TLS ALPN configuration failed".into());
    }

    runtime.block_on(wificalling_location_gateway::run_h2_prior_knowledge_smoke())?;
    println!("rust spike self-check passed");
    Ok(())
}
