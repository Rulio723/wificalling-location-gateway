# Multi-agent development contract

## Mission

Build `wificalling-location-gateway` as an isolated, fail-open OpenWrt component. Do not modify or vendor the stable Wi-Fi Calling Gateway 1.7 repository from this repository.

## Source of truth

1. A GitHub Issue is the only unit of assignable work.
2. `DEVELOPMENT_TEST_PLAN.md` defines architecture, safety gates, and phase exit criteria.
3. The Issue defines the owned paths, dependencies, acceptance tests, and non-goals.
4. A pull request is the only integration path into `main`.

## Agent roles and default ownership

| Role label | Default paths | Responsibility |
|---|---|---|
| `role:protocol` | `internal/wloc/`, `fixtures/` | Authorized fixtures, protocol notes, parser/patch behavior |
| `role:engine` | `cmd/`, `internal/ca/`, `internal/proxy/` | Process, TLS, HTTP/2, limits, fail-open behavior |
| `role:network` | `internal/exitprobe/`, `internal/georesolver/`, `openwrt/` | Exit probing, Geo resolution, nftables/dnsmasq/procd |
| `role:security` | `SECURITY.md`, `docs/security/`, `.github/` | Threat model, CA lifecycle, permissions, policy checks |
| `role:test` | `tests/`, `scripts/ci/` | Test harness, fuzzing, packaging and resource gates |
| `role:integration` | `docs/`, packaging metadata, Gateway contract | Cross-module contracts and release integration |

Issue-specific ownership overrides this table. An Agent must not edit another active Issue's owned paths without an explicit handoff recorded on both Issues.

## Required workflow

1. Claim one `status:ready` Issue and move it to `status:claimed`.
2. Create `codex/issue-<number>-<slug>` in an independent worktree.
3. Read this file, the Issue, and relevant sections of `DEVELOPMENT_TEST_PLAN.md`.
4. Write tests or executable verification before implementation when product code is in scope.
5. Keep commits focused and use Conventional Commits.
6. Open a PR containing `Closes #<number>`, evidence, risks, and rollback notes.
7. A different role reviews the PR. The author never self-approves a safety-sensitive change.

Use `scripts/agent-worktree.sh <issue> <agent> <slug>` to create a worktree and `scripts/claim-issue.sh <issue>` to claim the GitHub task.

## Hard gates

- Do not implement WLOC response patching before the Phase 0 authorized-fixture and license ADR Issues are closed.
- Never commit CA private keys, node credentials, captured device identifiers, raw production traffic, tokens, or precise user location.
- All parser and network inputs require size, time, concurrency, and schema limits.
- Unknown protocol, invalid Geo data, or engine failure must not produce a default fake coordinate.
- WLOC interception must remain limited to the assigned test device, two exact Apple hostnames, and TCP 443.
- Never intercept UDP 500/4500 or modify the Gateway 1.7 nftables table.
- Changes under `internal/ca/`, `internal/proxy/`, `openwrt/`, or `.github/workflows/` require security review.

## Verification

Run before every PR:

```sh
./scripts/ci/verify.sh
```

Product code must eventually meet the 80% coverage policy, but an empty scaffold does not fabricate coverage. Each Issue must state the tests appropriate to its phase.
