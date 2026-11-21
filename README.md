# Luna AI

Luna AI is a Rust/Leptos web application with a separate Axum API server.

## Repository layout

- Root `main.rs`, `api.rs`, `state.rs`, `ui.rs`, `style.css`: Leptos client (WASM).
- Root `data.json` and `data.rs`: bundled UI copy, themes, models, and static data.
- `backend/`: Axum server, Supabase Auth integration, chat-history API, and Groq/Gemini provider adapters.
- `docs/MODERNIZATION_PLAN.md`: architecture audit and modernization decisions.

## Build the client

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk build --release
```

The root `Cargo.toml` explicitly points the binary target to `main.rs`.

## Run the API server

Configure the server environment from `backend/.env.example`, then run:

```sh
cargo run --release --manifest-path backend/Cargo.toml
```

The API listens on `PORT` (default `3000`). The server requires `SUPABASE_URL` and `SUPABASE_ANON_KEY`.

## Restored API routes

- `POST /api/register`, `POST /api/login`, `GET /api/me`, `POST /api/logout`
- `GET /api/chats`, `PUT /api/chats/:id`, `DELETE /api/chats/:id`, `DELETE /api/chats`
- `POST /api/chat`
- `PUT /api/plan` rejects client-side plan changes; entitlements must be controlled by trusted billing logic.
- `GET /health`

## Supabase setup

Apply the SQL migrations in `backend/migrations/` to the confirmed Luna Supabase project, in numerical order. The chat-history API uses the caller's Supabase access token and depends on RLS policies restricting rows to `auth.uid()`.

Do not apply these migrations to a different Supabase project. The connected account currently does not have access to the previously identified Luna project.

## AI providers

The server-side adapters support Groq and Gemini. Provider credentials are never sent to the browser. The current draft reads credentials from server environment variables as an interim bridge. The agreed Vault-backed provider settings and in-app provider selector are not complete yet.

## Current limitations

- Supabase migrations have not been applied to a remote project.
- Refresh-token rotation is not implemented; the session cookie currently lasts one hour.
- Chat completion currently returns a plain text response; token-by-token streaming is not implemented yet.
- The server has an in-process limit of 12 chat requests per account per minute. This limit resets on server restart and is not distributed across replicas.
- CI must pass and the app must be integration-tested before production deployment.
