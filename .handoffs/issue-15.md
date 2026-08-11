# Agent handoff: Issue 15

## Identity and scope

- Source agent ID: codex-rust-migration
- Capabilities used: rust,tls-h2,openwrt,ci,security
- Branch: codex/issue-15-rust-migration
- Checkpoint parent: a68bc55693309629510a4f8c873b0cf80587740c
- Updated at (UTC): 2026-08-11T10:32:02Z
- Credentials included: no

## Objective

Establish reproducible target evidence for choosing Rust: a pinned OpenWrt
24.10.8 mt7622/AArch64 cross-build and a bounded, real TLS-over-H2 integration
test. This checkpoint does not implement WLOC parsing, CA installation, traffic
interception, packaging, or deployment.

## Completed

- Pinned Rust 1.90.0 and a minimal rustls/ring, Tokio, h2, and prost dependency
  spike with a checked-in lockfile and dependency policy.
- Added a real in-memory TLS handshake that verifies the approved hostname,
  requires ALPN `h2`, and completes one bounded H2 request and response.
- Added fail-closed tests for a wrong hostname, non-H2 ALPN, invalid H2 frame
  limits, and an oversized response body.
- Added a checksum- and digest-pinned OpenWrt 24.10.8 mt7622 cross-build whose
  compilation phase is network-disabled, locked, offline, and uses the OpenWrt
  linker, archiver, strip, and readelf tools.
- Recorded the final Cortex-A53 artifact as ELF64/AArch64, statically linked,
  and 1,118,352 bytes after stripping.
- Enforced Clippy, RustSec audit, cargo-deny, and at least 80% line coverage in
  repository verification.

## Files changed

- `.github/workflows/ci.yml`
- `Cargo.toml`, `Cargo.lock`, `deny.toml`
- `src/lib.rs`, `src/main.rs`, `src/tls_h2.rs`
- `tests/rust_spike_contract.rs`, `tests/rust_spike_policy.rs`,
  `tests/rust_tls_h2.rs`
- `scripts/ci/verify.sh`, `scripts/ci/verify-rust.sh`,
  `scripts/ci/verify-rust-openwrt.sh`
- `tests/scripts/test-verify-rust-openwrt.sh`
- `docs/testing/RUST_OPENWRT_CROSS_BUILD.md`
- `.handoffs/issue-15.md`

## Verification

| Command | Result | Evidence |
|---|---|---|
| `./scripts/ci/verify.sh` | Passed | 14 Rust tests, Clippy, audit/deny, secret scan, repository gates |
| `cargo llvm-cov --workspace --all-targets --locked --fail-under-lines 80` | Passed | 89.39% line coverage |
| `tests/scripts/test-verify-rust-openwrt.sh` | Passed | pinned-input, checksum, offline, cache-safety, size-boundary tests |
| `OPENWRT_CROSS_CACHE_DIR=/tmp/wloc-rust-openwrt-issue15 ./scripts/ci/verify-rust-openwrt.sh` | Passed | ELF64/AArch64, Cortex-A53, static, 1,118,352 bytes |

## Failed attempts

- The TLS test first produced the intended RED result: three behavior tests
  failed against `NotImplemented`; the final suite is six of six passing.
- The first target run used a generic AArch64 CPU and produced 1,118,344 bytes.
  It was superseded by the explicit Cortex-A53 build at 1,118,352 bytes.

## Unresolved decisions and blockers

- Compare static musl with OpenWrt dynamic libc before selecting production
  package linkage.
- Run the artifact under QEMU and on an authorized AX6S test router.
- Phase 0 fixture, protocol-contract, IPv6, and fail-open SLO gates remain
  prerequisites for WLOC parser, CA, MITM, or real-device interception work.
- The old local Go comparison scaffold is intentionally not part of this
  checkpoint and should be removed only in a separately reviewed migration
  step.

## Next executable steps

1. Review and merge this evidence-only migration checkpoint.
2. Open a separate packaging/QEMU Issue to compare linkage and measure runtime
   memory, startup, watchdog, and controlled-failure behavior.
3. Close the remaining Phase 0 gates before opening any protocol implementation
   Issue.

## Capabilities required for the next Agent

- rust
- openwrt
- ci
- security

## Environment assumptions

- The real cross-build host has Docker and can run a pinned linux/amd64 image.
- Bootstrap network access is permitted only before the separate
  `--network none` compilation phase.
- Agents authenticate independently and never exchange API keys.

## Security and privacy notes

- Runtime TLS test keys and certificates are generated in memory and are never
  written to the repository.
- No API keys, tokens, private keys, raw production captures, device
  identifiers, or precise user locations are included.
- No Gateway 1.7 table, UDP 500/4500 path, CA store, or device configuration is
  changed.
