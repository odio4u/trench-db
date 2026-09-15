---
name: Trust & Bootstrap Findings
about: Findings and implementation notes for node identity trust/bootstrap design
---

## Context

The repository's node identity and trust design needs explicit tasks to address bootstrap trust, credential verification, and stable identity handling. Current code creates identities and certificates but does not yet implement a credential verification/trust cache; the initial bootstrap remains a TOFU problem.

This item captures the security findings from a recent review and provides implementation notes to close the gaps.

## Implementation notes

- Add a `TrustCache` and `NodeCredential` types in `transport/src/tls/` (see `tls-implementation-plan.md`).
- Implement a `ServerCertVerifier` that extracts the credential from the certificate and validates it against the trust cache or trusted issuers.
- Persist node keys/certificates to stable paths (not regenerated on each startup); protect private keys with file permissions and consider optional OS-level secret stores.
- Define a bootstrap procedure: accept an out-of-band pinned issuer key or fingerprint (config), support N-of-M bootstrapping later.
- Replace UUID-in-config as the sole identity claim with a self-certifying model where `node_id` is derived from the node public key or validated by the issuer-signed credential.
- Add unit tests for credential verification, auth proof binding, and trust cache persistence.

## Related files

- `trench/src/identity/node_identity.rs`
- `trench/src/identity/certs.rs`
- `doc/communication/trust.md`
- `doc/transport/tls-implementation-plan.md`
