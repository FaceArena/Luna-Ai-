# Luna AI modernization — implementation plan

## Repository audit (2026-11-21)

The default branch currently contains a Leptos 0.6 WebAssembly client at the repository root:
- `main.rs`, `api.rs`, `state.rs`, `ui.rs`, `data.rs`, `data.json`, `style.css`, `index.html`, `Cargo.toml`, `Trunk.toml`.
- The client calls `/api/me`, `/api/login`, `/api/register`, `/api/logout`, `/api/chats`, `/api/plan`, and `/api/chat`.
- The README describes `backend/` and `frontend/` directories, but neither exists in the current Git tree. There is no Axum server implementation in this repository snapshot.
- Chat requests currently send `msgs`, `name`, `who`, `rules`, `len`, and `you` to `/api/chat`; the client consumes a streamed response body.
- Model selection is currently a local three-position selector (`mi`) tied to the bundled `data.json` model list. It is not a provider selector.
- The current README claims Claude server-side integration, Argon2 accounts, SQLite chat history, and rate limiting, but the implementation files for those features are absent from the current tree. Those claims must not be treated as verified.

## Target architecture

1. Keep the existing Leptos client and visual design.
2. Add a separate Rust server crate so browser/WASM dependencies remain isolated from server dependencies.
3. Implement a provider adapter interface with explicit provider/model configuration:
   - Groq Chat Completions API
   - Google Gemini API
   - Claude compatibility can be retained if its server implementation is recovered or rebuilt.
4. The server owns all provider calls. No provider key may be sent to the browser, returned by settings APIs, or logged.
5. Store provider secrets in Supabase Vault. Store only non-secret provider/model preferences in ordinary settings rows.
6. Require authenticated, owner-scoped settings operations and strict RLS. Never expose decrypted Vault views to anon/authenticated browser roles.
7. Preserve the existing streaming response contract consumed by `state.rs`.

## Blocking dependency

The connected Supabase account currently cannot access the previously identified project ref `catcugjmimfbxlthajmu`. The accessible projects are not confirmed to belong to Luna, so no database, Vault, or migration changes may be made there.

## Required validation before production

- Compile the client and server separately.
- Provider adapter tests for valid responses, malformed responses, upstream 401/403, 429, timeout, and streaming termination.
- Verify provider selection cannot be overridden by an untrusted client payload.
- Verify secrets never appear in API responses, browser storage, logs, or Git.
- Verify RLS and Vault permissions using non-owner and owner sessions.
- End-to-end test login, chat history, provider switching, and streamed chat.
