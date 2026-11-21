-- Luna AI provider preferences and Vault references.
-- Apply only to the confirmed Luna Supabase project.
-- This table stores a Vault UUID, never the plaintext provider key.

create table if not exists public.luna_provider_settings (
    user_id uuid primary key references auth.users(id) on delete cascade,
    provider text not null default 'groq'
        check (provider in ('groq', 'gemini')),
    model text not null,
    vault_secret_id uuid references vault.secrets(id) on delete set null,
    updated_at timestamptz not null default now()
);

alter table public.luna_provider_settings enable row level security;

-- Browser users may inspect their selected provider/model only.
-- They cannot read secret values or change the Vault reference directly.
drop policy if exists "Users can read their Luna provider preference"
    on public.luna_provider_settings;
create policy "Users can read their Luna provider preference"
    on public.luna_provider_settings
    for select to authenticated
    using (auth.uid() = user_id);

revoke all on public.luna_provider_settings from anon;
revoke insert, update, delete on public.luna_provider_settings from authenticated;
grant select on public.luna_provider_settings to authenticated;
grant all on public.luna_provider_settings to service_role;

-- Server-side code must use the service role only for the narrowly scoped
-- provider-secret RPCs introduced in a later migration. Never grant browser
-- roles access to vault.decrypted_secrets.
