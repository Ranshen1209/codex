---
title: Sakrylle CLI Implementation Status
status: local
scope: product-local
canonical_source: ../../sub2api/sakrylle-docs/10-platform-identity/current-state.md
last_verified: 2026-06-06
---

# Sakrylle CLI Implementation Status

Current documentation status: **partial: configuration isolation and Sakrylle provider are implemented; strict OIDC RP validation remains to be completed.**

Canonical platform status lives in [Sakrylle OIDC current state](../../sub2api/sakrylle-docs/10-platform-identity/current-state.md). This file only tracks product-local readiness and gaps.

## Product-local readiness checklist

- [ ] Local configuration points are documented in [local-integration.md](./local-integration.md).
- [ ] Product-specific OAuth/OIDC callback or scheme is documented.
- [ ] Token storage behavior is documented.
- [ ] Login, refresh, revoke/logout, and profile mapping smoke tests are documented.
- [ ] Known security gaps are linked to product implementation tasks.

## Suggested verification

- Verify `sakrylle` uses `~/.sakrylle-cli` rather than `~/.codex` by default.
- Run OAuth login against Sakrylle issuer in a non-production test profile.
- Confirm access tokens work for `/v1/responses` and usage is billed by Sakrylle API.
- Check logout/revoke clears local credentials and does not affect upstream Codex config.

