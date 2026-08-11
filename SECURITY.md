# Security policy

This repository implements a security-sensitive network experiment. Do not open a public Issue containing a vulnerability, credential, CA private key, raw packet capture, device identifier, node configuration, or precise user location.

Use the private GitHub Security Advisory form for vulnerability reports.

## Development rules

- Only synthetic or explicitly authorized, sanitized WLOC fixtures may enter Git.
- Generated CA private keys and leaf keys must remain on the test router with mode `0600`.
- Logs and CI artifacts must redact node credentials, tokens, device addresses, and request bodies.
- Proxy and parser code must enforce connection, stream, body, decompression, allocation, and timeout limits.
- A dead or unhealthy engine must cause its dedicated redirect to be removed, not leave traffic blackholed.
- Security-sensitive pull requests require review by someone other than the author.

No production deployment or emergency-service claim is supported during the PoC phase.
