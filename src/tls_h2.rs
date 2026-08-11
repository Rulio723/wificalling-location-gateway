//! Bounded in-memory TLS -> ALPN -> HTTP/2 integration harness.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::{Request, Response};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use tokio_rustls::{TlsAcceptor, TlsConnector};

const REQUIRED_ALPN: &[u8] = b"h2";
const REQUEST_PATH: &str = "/wloc-test";
const RESPONSE_BODY: &[u8] = b"h2-ok";

/// Limits applied to every in-memory TLS/HTTP/2 exchange.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlsH2Limits {
    pub io_capacity_bytes: usize,
    pub initial_window_bytes: u32,
    pub max_frame_bytes: u32,
    pub max_concurrent_streams: u32,
    pub max_response_body_bytes: usize,
    pub operation_timeout: Duration,
}

impl Default for TlsH2Limits {
    fn default() -> Self {
        Self {
            io_capacity_bytes: 64 * 1024,
            initial_window_bytes: 32 * 1024,
            max_frame_bytes: 16 * 1024,
            max_concurrent_streams: 1,
            max_response_body_bytes: 8 * 1024,
            operation_timeout: Duration::from_secs(2),
        }
    }
}

/// Ephemeral test-only certificate material supplied by the caller.
pub struct TlsH2TestMaterial {
    pub certificate_der: Vec<u8>,
    pub private_key_pkcs8_der: Vec<u8>,
}

/// Inputs that vary between the positive and fail-closed integration paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TlsH2ExchangeConfig {
    pub server_name: String,
    pub server_alpn_protocols: Vec<Vec<u8>>,
    pub limits: TlsH2Limits,
}

/// Evidence returned only after a request and response cross the H2 stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TlsH2ExchangeReport {
    pub negotiated_alpn: Vec<u8>,
    pub request_path: String,
    pub response_status: u16,
    pub response_body: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TlsH2Error {
    InvalidLimits,
    TlsHandshakeFailed,
    H2AlpnRequired,
    H2ProtocolFailed,
    ResponseBodyTooLarge,
    TimedOut,
}

impl std::fmt::Display for TlsH2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TlsH2Error {}

struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Run one real TLS handshake followed by one HTTP/2 request and response.
pub async fn run_in_memory_tls_h2_exchange(
    config: TlsH2ExchangeConfig,
    material: TlsH2TestMaterial,
) -> Result<TlsH2ExchangeReport, TlsH2Error> {
    validate_limits(config.limits)?;
    let timeout = config.limits.operation_timeout;

    tokio::time::timeout(timeout, run_bounded_exchange(config, material))
        .await
        .map_err(|_| TlsH2Error::TimedOut)?
}

fn validate_limits(limits: TlsH2Limits) -> Result<(), TlsH2Error> {
    const MIN_H2_FRAME_BYTES: u32 = 16 * 1024;
    const MAX_H2_FRAME_BYTES: u32 = (1 << 24) - 1;

    if limits.io_capacity_bytes < limits.max_frame_bytes as usize
        || limits.initial_window_bytes == 0
        || !(MIN_H2_FRAME_BYTES..=MAX_H2_FRAME_BYTES).contains(&limits.max_frame_bytes)
        || limits.max_concurrent_streams == 0
        || limits.max_response_body_bytes == 0
        || limits.operation_timeout.is_zero()
    {
        return Err(TlsH2Error::InvalidLimits);
    }
    Ok(())
}

async fn run_bounded_exchange(
    config: TlsH2ExchangeConfig,
    material: TlsH2TestMaterial,
) -> Result<TlsH2ExchangeReport, TlsH2Error> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let versions = [&rustls::version::TLS13, &rustls::version::TLS12];
    let certificate = CertificateDer::from(material.certificate_der);
    let private_key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(material.private_key_pkcs8_der));

    let mut server_config = rustls::ServerConfig::builder_with_provider(Arc::clone(&provider))
        .with_protocol_versions(&versions)
        .map_err(|_| TlsH2Error::TlsHandshakeFailed)?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], private_key)
        .map_err(|_| TlsH2Error::TlsHandshakeFailed)?;
    server_config.alpn_protocols = config.server_alpn_protocols;

    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(certificate)
        .map_err(|_| TlsH2Error::TlsHandshakeFailed)?;
    let mut client_config = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&versions)
        .map_err(|_| TlsH2Error::TlsHandshakeFailed)?
        .with_root_certificates(roots)
        .with_no_client_auth();
    // Offering a fallback lets the test prove that a TLS session which
    // negotiates something other than h2 is rejected before any H2 bytes flow.
    client_config.alpn_protocols = vec![REQUIRED_ALPN.to_vec(), b"http/1.1".to_vec()];

    let server_name =
        ServerName::try_from(config.server_name).map_err(|_| TlsH2Error::TlsHandshakeFailed)?;
    let (client_io, server_io) = tokio::io::duplex(config.limits.io_capacity_bytes);
    let server_handshake = TlsAcceptor::from(Arc::new(server_config)).accept(server_io);
    let client_handshake =
        TlsConnector::from(Arc::new(client_config)).connect(server_name, client_io);
    let (server_tls, client_tls) = tokio::join!(server_handshake, client_handshake);
    let server_tls = server_tls.map_err(|_| TlsH2Error::TlsHandshakeFailed)?;
    let client_tls = client_tls.map_err(|_| TlsH2Error::TlsHandshakeFailed)?;

    let negotiated_alpn = client_tls
        .get_ref()
        .1
        .alpn_protocol()
        .ok_or(TlsH2Error::H2AlpnRequired)?
        .to_vec();
    if negotiated_alpn != REQUIRED_ALPN
        || server_tls.get_ref().1.alpn_protocol() != Some(REQUIRED_ALPN)
    {
        return Err(TlsH2Error::H2AlpnRequired);
    }

    let mut server_builder = h2::server::Builder::new();
    server_builder
        .initial_window_size(config.limits.initial_window_bytes)
        .max_frame_size(config.limits.max_frame_bytes)
        .max_concurrent_streams(config.limits.max_concurrent_streams)
        .max_concurrent_reset_streams(config.limits.max_concurrent_streams as usize)
        .max_pending_accept_reset_streams(config.limits.max_concurrent_streams as usize);
    let mut client_builder = h2::client::Builder::new();
    client_builder
        .initial_window_size(config.limits.initial_window_bytes)
        .max_frame_size(config.limits.max_frame_bytes)
        .max_concurrent_streams(config.limits.max_concurrent_streams)
        .max_concurrent_reset_streams(config.limits.max_concurrent_streams as usize)
        .max_pending_accept_reset_streams(config.limits.max_concurrent_streams as usize);

    let (server_connection, client_connection) = tokio::join!(
        server_builder.handshake::<_, Bytes>(server_tls),
        client_builder.handshake::<_, Bytes>(client_tls)
    );
    let mut server_connection = server_connection.map_err(|_| TlsH2Error::H2ProtocolFailed)?;
    let (mut request_sender, client_driver) =
        client_connection.map_err(|_| TlsH2Error::H2ProtocolFailed)?;
    let mut client_driver = AbortOnDrop(tokio::spawn(client_driver));

    let request = Request::builder()
        .method("GET")
        .uri(format!("https://localhost{REQUEST_PATH}"))
        .body(())
        .map_err(|_| TlsH2Error::H2ProtocolFailed)?;
    let (response_future, _request_body) = request_sender
        .send_request(request, true)
        .map_err(|_| TlsH2Error::H2ProtocolFailed)?;

    let (request, mut responder) = server_connection
        .accept()
        .await
        .ok_or(TlsH2Error::H2ProtocolFailed)?
        .map_err(|_| TlsH2Error::H2ProtocolFailed)?;
    let request_path = request.uri().path().to_owned();
    let response = Response::builder()
        .status(200)
        .body(())
        .map_err(|_| TlsH2Error::H2ProtocolFailed)?;
    let mut response_stream = responder
        .send_response(response, false)
        .map_err(|_| TlsH2Error::H2ProtocolFailed)?;
    response_stream
        .send_data(Bytes::from_static(RESPONSE_BODY), true)
        .map_err(|_| TlsH2Error::H2ProtocolFailed)?;

    // Poll the server connection to flush its queued DATA frame. This task is
    // cancelled only after the client verifies the complete response, so an
    // arbitrary BrokenPipe is never accepted as success evidence.
    let mut server_driver = AbortOnDrop(tokio::spawn(async move {
        while let Some(next_stream) = server_connection.accept().await {
            next_stream.map_err(|_| TlsH2Error::H2ProtocolFailed)?;
        }
        Ok::<(), TlsH2Error>(())
    }));

    let mut response = response_future
        .await
        .map_err(|_| TlsH2Error::H2ProtocolFailed)?;
    let response_status = response.status().as_u16();
    let mut response_body = Vec::new();
    while let Some(chunk) = response.body_mut().data().await {
        let chunk = chunk.map_err(|_| TlsH2Error::H2ProtocolFailed)?;
        if response_body.len().saturating_add(chunk.len()) > config.limits.max_response_body_bytes {
            return Err(TlsH2Error::ResponseBodyTooLarge);
        }
        response_body.extend_from_slice(&chunk);
    }

    // Holding the sender prevents normal client shutdown from racing with the
    // response checks. A driver that already stopped is a protocol failure;
    // otherwise both live drivers receive an explicit bounded cancellation.
    if client_driver.0.is_finished() || server_driver.0.is_finished() {
        return Err(TlsH2Error::H2ProtocolFailed);
    }
    drop(request_sender);
    server_driver.0.abort();
    client_driver.0.abort();
    let _ = (&mut server_driver.0).await;
    let _ = (&mut client_driver.0).await;

    Ok(TlsH2ExchangeReport {
        negotiated_alpn,
        request_path,
        response_status,
        response_body,
    })
}
