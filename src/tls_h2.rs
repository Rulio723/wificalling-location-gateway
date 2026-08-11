//! Bounded in-memory TLS -> ALPN -> HTTP/2 integration harness.

use std::time::Duration;

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
#[derive(Clone, Debug, Eq, PartialEq)]
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
    NotImplemented,
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

/// Run one real TLS handshake followed by one HTTP/2 request and response.
pub async fn run_in_memory_tls_h2_exchange(
    _config: TlsH2ExchangeConfig,
    _material: TlsH2TestMaterial,
) -> Result<TlsH2ExchangeReport, TlsH2Error> {
    Err(TlsH2Error::NotImplemented)
}
