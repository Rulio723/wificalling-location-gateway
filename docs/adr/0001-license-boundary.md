# ADR-0001: License and clean-room implementation boundary

- Status: Accepted for offline scaffolding only — protocol/consistency and independent security reviews completed; no parser, patch, CA, MITM, fixture, or live-traffic authorization
- Date: 2026-08-11
- Decision owners: protocol and security roles
- Phase: 0
- Supersedes: none
- Review record: [Phase 0 review](../reviews/PHASE0_OFFLINE_SCAFFOLD_REVIEW.md)

## Notice

This record is an engineering and contribution policy, not legal advice or a
guarantee of non-infringement. Copyright scope and license obligations depend
on facts and jurisdiction. Material uncertainty must be escalated to the
copyright holders or qualified counsel before release.

## Context

`wificalling-location-gateway` is an isolated component intended to integrate
with Wi-Fi Calling Gateway through documented configuration and data
contracts. Wi-Fi Calling Gateway is licensed under MIT. The protocol reference
identified by the development plan, `mekos2772/ios-location-spoofer`, declares
AGPL-3.0 and contains the full GNU Affero General Public License version 3 in
its `LICENSE` file.

The MIT license permits reuse subject to preservation of its notice. It does
not remove conditions attached to code received under another license. The
GNU project's compatibility guidance explains that compatible permissive code
may be combined into a GNU-licensed work, but the combination is released
under the applicable GNU license. AGPLv3 section 13 additionally requires a
modified version that supports remote network interaction to prominently
offer its Corresponding Source to remote users.

Repository or process separation alone does not prove that one program is
independent of another, and it does not relicense copied or derived code.
Accordingly, this ADR establishes both an integration boundary and a
provenance-controlled implementation process.

## Decision

This project selects the **independent clean-room implementation route**.

The WLOC parser, response patcher, fixtures, protocol notes, tests, and related
interfaces must be created without copying, adapting, translating, or using
the structure of the AGPL reference implementation. The reference repository
is not a source dependency, build dependency, test dependency, submodule,
vendored artifact, or prompt/context input for implementing those components.

The intended publication outcome is an original implementation that can use a
permissive, Gateway-compatible license such as MIT. This ADR does not itself
grant a license: until a repository `LICENSE` is added after ownership and
notice review, the repository remains **all rights reserved by default** and
must not be described as MIT-licensed.

Wi-Fi Calling Gateway remains a separately versioned MIT project. Integration
is limited to documented configuration, process control, and data contracts.
The location component must not copy or vendor Gateway source merely for
convenience; any future reuse of Gateway code must retain its MIT notice and be
recorded in the provenance ledger.

## Inputs permitted for clean-room work

An implementer may use only inputs whose origin and authorization are recorded:

1. Publicly available standards, vendor documentation, and library API
   documentation, with stable URLs, titles, versions, and access dates.
2. Facts learned from authorized black-box interoperability testing, expressed
   as original observations rather than copied prose, diagrams, identifiers,
   control flow, data structures, or test cases from the reference source.
3. Synthetic fixtures created from an independently written schema or test
   requirement.
4. Explicitly authorized device captures that have passed the separate fixture
   governance process, sanitization review, and security review.
5. Protocol notes written by a reference-side observer from allowed behavioral
   observations, provided they contain no source excerpts, source-derived
   pseudocode, original comments, distinctive names, file layout, or
   implementation suggestions.
6. Original contributions whose authors attest to provenance and disclose
   relevant prior access.
7. MIT Gateway documentation and, only after explicit provenance review, MIT
   Gateway code used in compliance with its copyright notice and permission
   terms.

Use of public documentation is subject to that material's own license and
terms. A public URL does not by itself make expressive content safe to copy.

## Inputs prohibited for clean-room work

The following must not enter implementation branches, Issues, pull requests,
fixtures, protocol notes, AI prompts, generated artifacts, or review comments:

- source code, patches, diffs, binaries, source maps, generated output, tests,
  fixtures, comments, documentation text, screenshots of source, or repository
  structure copied from the AGPL reference implementation;
- line-by-line or language-to-language translation, paraphrased pseudocode,
  distinctive algorithms, names, constants, tables, schemas, or test vectors
  derived from inspection of that source;
- decompiled or disassembled reference artifacts, or output produced by tools
  whose purpose is to reconstruct the reference implementation;
- third-party summaries that reproduce or closely transform protected source;
- raw production traffic, unauthorized captures, device identifiers, BSSID or
  cell observations, credentials, CA private keys, precise user location, or
  any fixture that lacks documented capture authority and sanitization;
- claims that separate repositories, processes, RPC, dynamic linking, or an
  API boundary automatically avoid AGPL or copyright obligations.

Copying AGPL-covered material is not made acceptable by adding an MIT header.
If AGPL material is intentionally reused, this clean-room decision no longer
applies and the change must follow the exception process below before work
continues.

## Contributor and Agent separation

The workflow has two mutually exclusive protocol roles:

### Reference-side observer

A reference-side observer may inspect the AGPL repository only for a separately
approved compatibility investigation. They may produce a factual behavioral
specification using allowed observations, but must not write or review the
clean-room WLOC parser, patcher, protocol fixtures, or protocol-specific tests.
Their deliverable requires protocol and security review for source leakage.

### Clean-room implementer

A clean-room implementer must not inspect the AGPL source, tests, fixtures,
commits, pull requests, or source-derived descriptions for this work. Before
accepting a protocol Issue, the contributor or Agent records:

- a non-secret contributor or Agent ID;
- the Issue and owned paths;
- the allowed specifications and fixture identifiers used;
- whether they previously accessed the reference source and, if so, enough
  detail for reviewers to decide whether reassignment is required;
- an attestation that no prohibited input was used or supplied to an AI tool.

Prior exposure does not imply misconduct, but protocol implementation must be
reassigned when the protocol and security reviewers cannot establish a
credible independent path. Exposed contributors may work on unrelated areas
such as generic CI or networking where the Issue excludes WLOC behavior.

AI Agents are contributors for this policy. Project-provided context for an AI
implementer must contain only approved specifications and sanitized fixtures;
AGPL excerpts or source-derived prompts invalidate the clean-room workflow.

No person may serve as both reference-side observer and clean-room implementer
for the same protocol behavior. A reviewer who inspected reference source may
review provenance and licensing, but must not suggest source-derived code.

## Fixture and protocol-note provenance

Every committed fixture and protocol note must be traceable without retaining
sensitive capture data. Its adjacent manifest or review record must contain:

- a stable artifact ID and cryptographic digest;
- artifact kind: fixture manifests use only `synthetic` or
  `authorized-sanitized-capture`; protocol-note review records may additionally
  use `public-document-observation`, which is never a fixture category;
- creator/observer ID and creation date;
- capture authority or synthetic generation method;
- source title, version, and URL for public documentation;
- sanitization actions and confirmation that prohibited sensitive fields are
  absent;
- reviewers from both protocol and security roles;
- a statement that the artifact was not copied or derived from AGPL source;
- retention and deletion rules for any uncommitted raw material.

Raw authorized captures remain outside Git and outside shared Agent context.
Only the minimum sanitized fixture needed for a named test may be committed.
Protocol notes describe externally observable fields, constraints, and expected
behavior; they must not describe how the reference implementation realizes
that behavior.

An artifact with missing, ambiguous, or disputed provenance is quarantined and
must not be used to design code, tests, or schemas. Its dependent work returns
to blocked status until reviewers approve a replacement.

## Review and enforcement

Before protocol implementation starts, protocol and security reviewers must
confirm all of the following:

- this ADR is accepted and the repository license status is still stated
  accurately;
- the fixture governance document is accepted;
- the initial protocol specification identifies only allowed sources;
- every initial fixture has complete provenance metadata;
- Issue ownership keeps observer and implementer roles separate;
- dependency and repository scans contain no reference package, copied file,
  suspicious source phrase, or AGPL artifact;
- the pull request records the contributor provenance attestation.

Automated similarity and dependency scans are supporting controls, not proof
of independent creation. Human protocol and security review remains required.

## Exceptions and future changes

There is no informal exception. A request to reuse AGPL material, relax role
separation, change the intended project license, or combine the component with
another work must:

1. stop affected implementation and quarantine the proposed material;
2. open a dedicated Issue identifying the exact material, copyright holder,
   license version, combination/deployment model, and affected paths;
3. obtain written permission from the relevant copyright holder or a reviewed
   compliance plan for adopting AGPL-3.0 obligations, including source notices,
   Corresponding Source, build/install materials where applicable, and the
   section 13 network source offer;
4. receive approval from protocol, security, and repository ownership roles,
   with qualified legal review when material uncertainty remains;
5. supersede this ADR and update `LICENSE`, notices, documentation, packaging,
   CI policy, and release procedures before the material is used.

A future decision to adopt AGPL is valid only prospectively after that process;
it must not be used to retroactively legitimize undocumented provenance.

## Consequences

### Benefits

- Keeps the stable MIT Gateway and the new component operationally and
  provenance-wise distinct.
- Preserves the option of a permissive license for original work.
- Makes fixture authorization and protocol evidence auditable across Agent
  handoffs.
- Avoids accidental reliance on AGPL source while the project is still in
  Phase 0.

### Costs and limitations

- Protocol discovery and implementation take longer and require separate
  people or Agents.
- Some contributors with prior source exposure cannot implement the affected
  protocol behavior.
- Provenance records and dual review are mandatory.
- Clean-room process reduces risk but is not a legal safe harbor or guarantee.

## Phase 0 exit effect

Acceptance of this ADR resolves only the license-boundary portion of Phase 0.
It does not authorize WLOC parser or response-patch implementation until the
authorized fixture contract and WLOC threat model are also accepted. No real
device interception is authorized by this decision.

After all three Phase 0 documents are accepted, the next allowed scope is
limited to a Go module, an offline manifest validator, CI scaffolding, and
generic protocol safety-contract tests. That scope must contain no Apple
private field numbers or semantics, real capture bytes, response-patch logic,
CA generation, MITM, or live traffic.

## Authoritative references checked

- [GNU Affero General Public License v3, including section 13](https://www.gnu.org/licenses/agpl-3.0.html)
- [GNU license FAQ: compatibility, combinations, and AGPL Corresponding Source](https://www.gnu.org/licenses/gpl-faq.html)
- [GNU license compatibility guidance](https://www.gnu.org/licenses/license-compatibility.html)
- [Open Source Initiative: MIT License](https://opensource.org/license/mit)
- [`mekos2772/ios-location-spoofer` repository](https://github.com/mekos2772/ios-location-spoofer)
- [`ios-location-spoofer` AGPL-3.0 license at evidence commit `b72d6f67efb2b457647ae05e3e20ae3f3f6f0262`](https://github.com/mekos2772/ios-location-spoofer/blob/b72d6f67efb2b457647ae05e3e20ae3f3f6f0262/LICENSE)
- [Wi-Fi Calling Gateway repository](https://github.com/smthdagg/luci-app-wificalling-gateway) (private project source; local `LICENSE` reviewed as MIT)
