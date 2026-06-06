---
title: Sakrylle CLI Local Integration
status: local
scope: product-local
canonical_source: ../../sub2api/sakrylle-docs/10-platform-identity/rp-integration-guide.md
last_verified: 2026-06-06
---

# Sakrylle CLI Local Integration

This page summarizes only the repository-local OIDC/Sakrylle integration concerns for **Sakrylle CLI**.

For protocol details, use the canonical [RP integration guide](../../sub2api/sakrylle-docs/10-platform-identity/rp-integration-guide.md). For current Sakrylle API/OIDC Provider capability, use [current-state.md](../../sub2api/sakrylle-docs/10-platform-identity/current-state.md).

## Local focus areas

- `SAKRYLLE_CLI_HOME` and default `~/.sakrylle-cli` isolation
- `sakrylle` / `skl` binary packaging and command naming
- Responses API compatibility with Sakrylle API
- Codex auth.json / app-server compatibility
- Loopback login, Device Flow path compatibility, refresh and revoke behavior
- id_token validation gaps: nonce, JWKS signature, issuer, audience, expiry

## Preserved historical notes

Detailed original research and development planning were preserved under:

- [historical/research.md](./historical/research.md)
- [historical/development-plan.md](./historical/development-plan.md)

Those files are historical/product-local references. They do not override center platform facts.
