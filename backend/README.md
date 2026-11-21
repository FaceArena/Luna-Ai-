# Luna server

This crate is the server-side foundation for Luna's provider integration.

## Current state

- Axum health endpoint: `GET /health`.
- Server-only Groq and Gemini request adapters in `src/providers.rs`.
- The provider adapters currently implement non-streaming completions.
- No chat route is exposed yet. This is intentional: the repository does not currently contain the authentication/session backend described by the root README. Exposing a chat endpoint before restoring authenticated sessions would create an unauthenticated, billable API.
- No Supabase project migration has been applied. Confirm the Luna project before applying `migrations/0001_provider_settings.sql`.

## Run

```sh
cargo run --manifest-path backend/Cargo.toml
```

The health endpoint listens on `PORT` (default `3000`).

## Security requirements before enabling chat

1. Restore or implement the account/session API and validate the existing HttpOnly session cookie on every chat/settings request.
2. Resolve provider/model preferences for the authenticated user on the server; ignore provider selection sent in a chat payload.
3. Store each provider API key in Supabase Vault. Store only the Vault secret UUID in `luna_provider_settings`.
4. Read decrypted Vault values only on the trusted server using a server-only credential. Never expose service-role keys or decrypted Vault data to the browser.
5. Add per-user rate limits, request-size limits, upstream timeouts, and safe error mapping.
6. Implement SSE/stream normalization for both upstream APIs and preserve the exact plain-text streaming contract expected by the existing Leptos client.
7. Add integration tests against a disposable Supabase project before deploying.

Provider adapter models must be explicit and validated against the provider's current supported model list. Do not infer paid entitlements from the old three-option UI.
