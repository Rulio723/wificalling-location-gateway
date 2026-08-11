# Multi-Agent workflow

## Control plane

GitHub Issues are the task queue. Labels express role, phase, and state; Milestones express release gates. The repository avoids a custom scheduler so humans and Agents see the same auditable state.

Task states:

```text
status:triage -> status:ready -> status:claimed -> status:review -> status:done
                                  |                   |
                                  +-> status:blocked <-+
```

Only tasks with explicit owned paths, dependencies, acceptance criteria, non-goals, and rollback behavior may become `status:ready`.

## Parallel work lanes

| Lane | Can start now | Gate |
|---|---|---|
| Protocol evidence | fixture schema, capture sanitization, protocol ADR | authorized samples required for parser implementation |
| Security | threat model, CA lifecycle ADR, secret scanning policy | independent review required |
| Exit/Geo | provider interfaces, schema and cache tests | no dependency on WLOC patch code |
| OpenWrt | nftables namespace and rollback design | no live redirect before IPv6 decision |
| Test infrastructure | CI, fuzz harness skeleton, resource measurement | fixtures remain synthetic until authorized |
| Engine | interfaces and limits ADR only | implementation blocked by protocol and license ADRs |

## Starting an Agent

1. Select one Issue carrying `status:ready` and one `role:*` label.
2. Claim it:

   ```sh
   ./scripts/claim-issue.sh 12
   ```

3. Create an isolated worktree:

   ```sh
   ./scripts/agent-worktree.sh 12 protocol fixture-contract
   ```

4. Start the Agent with the Issue URL, worktree path, owned paths, and instruction not to revert other Agents.
5. Push the branch and open a PR. Move the Issue to `status:review`.

## Handoffs

If a task discovers work outside its owned paths, it opens a new Issue. Cross-role contracts must be committed as schema, interface, fixture, or ADR before dependent implementation begins. Chat messages are coordination hints, not durable requirements.

## Merge policy

- Required checks: `verify` and `pull-request-contract`.
- At least one approving review.
- Code owner review for CA, proxy, OpenWrt, or workflow paths.
- Resolve all review threads and preserve linear history.
- Delete merged branches; prune worktrees locally after merge.

## Current GitHub account limitation

The repository is private. GitHub rejected server-side branch protection for the current account tier, so CI, CODEOWNERS, and this contract are active but cannot technically prevent the repository owner from pushing directly to `main`. Upgrade the owner to GitHub Pro before adding autonomous write-capable Agents if hard enforcement is required.
