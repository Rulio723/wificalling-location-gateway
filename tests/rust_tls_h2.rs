use rcgen::{generate_simple_self_signed, CertifiedKey};
use wificalling_location_gateway::tls_h2::{
    run_in_memory_tls_h2_exchange, TlsH2Error, TlsH2ExchangeConfig, TlsH2Limits,
    TlsH2TestMaterial,
};

const APPROVED_HOSTNAME: &str = "gs-loc.apple.com";

fn ephemeral_material() -> TlsH2TestMaterial {
    // The only SAN is the approved hostname. Both key and certificate exist
    // only in memory for the duration of a test and are never written to disk.
    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed([APPROVED_HOSTNAME.to_owned()])
            .expect("runtime certificate generation must succeed");

    TlsH2TestMaterial {
        certificate_der: cert.der().to_vec(),
        private_key_pkcs8_der: signing_key.serialize_der(),
    }
}

fn exchange_config(server_name: &str, server_alpn: &[&[u8]]) -> TlsH2ExchangeConfig {
    TlsH2ExchangeConfig {
        server_name: server_name.to_owned(),
        server_alpn_protocols: server_alpn.iter().map(|value| value.to_vec()).collect(),
        limits: TlsH2Limits::default(),
    }
}

#[tokio::test]
async fn approved_hostname_and_h2_complete_request_response_on_one_tls_path() {
    let report = run_in_memory_tls_h2_exchange(
        exchange_config(APPROVED_HOSTNAME, &[b"h2"]),
        ephemeral_material(),
    )
    .await
    .expect("valid hostname and h2 ALPN must complete the exchange");

    assert_eq!(report.negotiated_alpn, b"h2");
    assert_eq!(report.request_path, "/wloc-test");
    assert_eq!(report.response_status, 200);
    assert_eq!(report.response_body, b"h2-ok");
}

#[tokio::test]
async fn hostname_outside_certificate_san_fails_closed_during_tls() {
    let error = run_in_memory_tls_h2_exchange(
        exchange_config("unapproved.invalid", &[b"h2"]),
        ephemeral_material(),
    )
    .await
    .expect_err("a hostname outside the certificate SAN must not reach H2");

    assert_eq!(error, TlsH2Error::TlsHandshakeFailed);
}

#[tokio::test]
async fn server_without_h2_alpn_fails_closed_before_h2_handshake() {
    let error = run_in_memory_tls_h2_exchange(
        exchange_config(APPROVED_HOSTNAME, &[b"http/1.1"]),
        ephemeral_material(),
    )
    .await
    .expect_err("a TLS connection without negotiated h2 must be closed");

    assert_eq!(error, TlsH2Error::H2AlpnRequired);
}

#[test]
fn resource_limits_are_explicit_and_bounded() {
    let limits = TlsH2Limits::default();

    assert_eq!(limits.io_capacity_bytes, 64 * 1024);
    assert_eq!(limits.initial_window_bytes, 32 * 1024);
    assert_eq!(limits.max_frame_bytes, 16 * 1024);
    assert_eq!(limits.max_concurrent_streams, 1);
    assert_eq!(limits.max_response_body_bytes, 8 * 1024);
    assert_eq!(limits.operation_timeout, std::time::Duration::from_secs(2));
}
