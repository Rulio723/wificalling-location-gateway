# Rust OpenWrt cross-build verification

Date: 2026-08-11  
Issue: #15  
Scope: reproducible Rust dependency-spike build for OpenWrt 24.10.8
`mediatek/mt7622`; this does not authorize WLOC protocol implementation,
traffic interception, CA generation, packaging, or deployment.

## Reproducibility contract

`scripts/ci/verify-rust-openwrt.sh` pins all identity-bearing build inputs:

- OpenWrt release: `24.10.8`
- Target: `mediatek/mt7622`
- Toolchain archive:
  `openwrt-toolchain-24.10.8-mediatek-mt7622_gcc-13.3.0_musl.Linux-x86_64.tar.zst`
- Toolchain URL:
  `https://downloads.openwrt.org/releases/24.10.8/targets/mediatek/mt7622/openwrt-toolchain-24.10.8-mediatek-mt7622_gcc-13.3.0_musl.Linux-x86_64.tar.zst`
- Toolchain SHA-256:
  `fc045488375d0ff6fe6bbd0d40db44b5faced186b3e8919a400d92867171a9ad`
- Build image:
  `rust:1.90.0-slim-bookworm@sha256:64232e656c058f4468e8d024e990acff04f0fd5a5c0a88a574dc37773d7325c9`
- Rust toolchain/target: `1.90.0` / `aarch64-unknown-linux-musl`
- Target CPU: `-C target-cpu=cortex-a53`, matching mt7622/AX6S
- Cargo resolution: checked-in `Cargo.lock`, used with `--locked` in both
  preparation and build phases.

The script has two deliberately separate container phases:

1. Bootstrap may use the network to install the archive extractor, install the
   pinned Rust standard-library target, and populate the locked Cargo cache.
   The downloaded OpenWrt archive is accepted only after SHA-256 verification.
2. Compilation runs in a new container with `--network none`, `--pull never`,
   read-only source, and `cargo build --offline --locked`. It uses the OpenWrt
   linker, archiver, strip, and readelf tools from the verified archive.

The bootstrap's Debian package index is not a byte-for-byte reproducible input;
it is used only to extract the checksum-pinned archive. It is outside the
network-disabled compilation boundary. Cargo crate content remains constrained
by `Cargo.lock` and Cargo registry checksums.

## Running the gate

Fetch the already pinned image explicitly, then run the verifier:

```sh
docker pull --platform linux/amd64 \
  rust:1.90.0-slim-bookworm@sha256:64232e656c058f4468e8d024e990acff04f0fd5a5c0a88a574dc37773d7325c9
OPENWRT_CROSS_CACHE_DIR=/tmp/wloc-rust-openwrt-issue15 \
  ./scripts/ci/verify-rust-openwrt.sh
```

The cache must be an absolute, dedicated path whose final component begins
with `wloc-rust-openwrt-`. Empty, relative, broad, home, repository, dot-path,
and symlink targets are rejected before network or Docker operations.

Build products stay outside the repository under
`$OPENWRT_CROSS_CACHE_DIR/output/`. Do not commit the binary, expanded
toolchain, Rust target component, Cargo cache, or generated report.

The `openwrt-cross-build` GitHub Actions job runs this same real build for every
pull request and push to `main`. The job log is the review-bound record of the
ELF header, dependency status, and stripped size; a fake-tool test alone cannot
satisfy the merge gate.

## TDD evidence

The user journey was derived from Issue #15: as an integration reviewer, I want
one command to reproduce and inspect the AX6S-class Rust artifact so that the
language migration is based on measurable target evidence.

RED command:

```sh
/bin/sh tests/scripts/test-verify-rust-openwrt.sh
```

Initial result: `FAIL: missing executable .../scripts/ci/verify-rust-openwrt.sh`.
This was the intended failure for the missing implementation.

GREEN commands:

```sh
/bin/sh -n scripts/ci/verify-rust-openwrt.sh
/bin/sh tests/scripts/test-verify-rust-openwrt.sh
OPENWRT_CROSS_CACHE_DIR=/tmp/wloc-rust-openwrt-issue15 \
  ./scripts/ci/verify-rust-openwrt.sh
```

Results on 2026-08-11:

- fake-tool boundary suite: `verify-rust-openwrt tests passed`
- target header: `ELF64`, `AArch64`
- dynamic dependency status: `NEEDED: none (statically linked)`
- final mt7622 `cortex-a53` stripped size: `1,118,352 bytes`, below the
  `8 MiB` gate
- real network-disabled, locked/offline compilation: passed

| Guarantee | Evidence | Type | Result |
|---|---|---|---|
| Fixed archive URL and SHA-256 are used | happy-path fake `curl`/`shasum` assertions | contract | PASS |
| A missing pinned image cannot trigger an implicit pull | missing-image case | negative | PASS |
| Checksum failure stops before any container run | bad-SHA case | negative | PASS |
| Dangerous cache paths stop before curl or Docker | dangerous-cache case | security | PASS |
| Final compilation is offline and lockfile-bound | fake Docker argument assertions | contract | PASS |
| Artifact is stripped with the target toolchain | fake Docker argument assertions | contract | PASS |
| Artifacts larger than 8 MiB fail | oversize case | boundary | PASS |
| Real output is an AArch64 ELF with recorded dependency status | OpenWrt readelf report | integration | PASS |

## Known gaps

- The binary has not yet run under QEMU or on the target router.
- This gate does not create an OpenWrt package or validate procd integration.
- Static musl linkage is intentional for this spike because it yields a single
  measurable artifact and avoids accidental dependence on the build image.
  Before production packaging, a separate gate must compare this choice with
  OpenWrt dynamic libc linkage for size, security updates, ABI compatibility,
  and package-policy impact. Any dependency or linkage change must rerun this
  gate.
- This shell contract suite exercises every policy branch added here, but it is
  not a line-coverage measurement. Product Rust coverage remains a separate
  80% gate.
