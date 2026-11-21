# Luna API server

This Axum service restores the API surface consumed by the existing Leptos client.

## Implemented routes

| Method | Route | Purpose |
|---|---|---|
| POST | `/api/register` | Create Supabase Auth account |
| POST | `/api/login` | Password sign-in |
| GET | `/api/me` | Validate the HttpOnly session cookie |
| POST | `/api/logout` | Clear the session cookie |
| GET | `/api/chats` | List current user's chat history |
| PUT | `/api/chats/:id` | Upsert a chat |
| DELETE | `/api/chats/:id` | Delete one chat |
| DELETE | `/api/chats` | Delete all current user's chats |
| POST | `/api/chat` | Authenticated AI completion |
| PUT | `/api/plan` | Save the demo plan choice (no billing) |
| GET | `/health` | Health check |

## Required Supabase configuration

Set `SUPABASE_URL` and `SUPABASE_ANON_KEY` in the server environment. The browser never receives a service-role key. Supabase Auth issues a short-lived access token stored in an HttpOnly, Secure, SameSite=Lax cookie. Supabase REST requests use that user's JWT, so database RLS policies scope chat access to `auth.uid()`.

Apply migrations in order to the confirmed Luna Supabase project:

1. `migrations/0001_provider_settings.sql`
2. `migrations/0002_chat_history.sql`

Do not apply these migrations to an unrelated Supabase project.

## Provider configuration status

The chat endpoint currently reads a provider key from the server environment and supports Groq or Gemini. This is an interim server-only bridge, not the final agreed configuration. The next security step is to add authenticated provider settings endpoints backed by Supabase Vault, then remove environment-key fallback. No provider key is returned to the browser.

## Important limitations

- Access tokens are short-lived; refresh-token rotation is not implemented yet.
- Chat responses are returned as text bodies; token-by-token SSE streaming is not implemented.
- Provider settings UI and Vault secret management are not yet connected.
- Run CI and integration tests before deployment. No migration has been applied remotely by this change.
