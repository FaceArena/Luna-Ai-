-- Luna AI provider preferences and Vault references.
-- Apply only to the confirmed Luna Supabase project after reviewing the project ref.
-- Secret values are stored in Supabase Vault; this table stores references only.

create table if not exists public.luna_provider_settings (
    user_id uuid primary key references auth.users(id) on delete cascade,
    provider text not null default 'groq'
        check (provider in ('groq', 'gemini')),
    model text not null,
    vault_secret_id uuid references vault.secrets(id) on delete set null,
    updated_at timestamptz not null default now()
);

alter table public.luna_provider_settings enable row level security;

drop policy if exists "Users can read their Luna provider preference"
    on public.luna_provider_settings;
create policy "Users can read their Luna provider preference"
    on public.luna_provider_settings
    for select to authenticated
    using (auth.uid() = user_id);

drop policy if exists "Users can create their Luna provider preference"
    on public.luna_provider_settings;
create policy "Users can create their Luna provider preference"
    on public.luna_provider_settings
    for insert to authenticated
    with check (auth.uid() = user_id);

drop policy if exists "Users can update their Luna provider preference"
    on public.luna_provider_settings;
create policy "Users can update their Luna provider preference"
    on public.luna_provider_settings
    for update to authenticated
    using (auth.uid() = user_id)
    with check (auth.uid() = user_id);

-- No browser role receives access to vault.decrypted_secrets.
-- The trusted server uses its server-only Supabase credential to resolve
-- vault_secret_id and read the secret at request time. Never return that value.
