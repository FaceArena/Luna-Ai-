-- Per-user Luna chat history. Supabase Auth JWT supplies auth.uid().
create table if not exists public.luna_chats (
    user_id uuid not null default auth.uid() references auth.users(id) on delete cascade,
    id bigint not null,
    title text not null default '',
    msgs jsonb not null default '[]'::jsonb,
    updated_at timestamptz not null default now(),
    primary key (user_id, id),
    constraint luna_chats_title_length check (char_length(title) <= 160),
    constraint luna_chats_msgs_array check (jsonb_typeof(msgs) = 'array')
);

create index if not exists luna_chats_updated_at_idx
    on public.luna_chats (user_id, updated_at desc);

alter table public.luna_chats enable row level security;

drop policy if exists "Users can read their Luna chats" on public.luna_chats;
create policy "Users can read their Luna chats"
    on public.luna_chats for select to authenticated
    using (auth.uid() = user_id);

drop policy if exists "Users can create their Luna chats" on public.luna_chats;
create policy "Users can create their Luna chats"
    on public.luna_chats for insert to authenticated
    with check (auth.uid() = user_id);

drop policy if exists "Users can update their Luna chats" on public.luna_chats;
create policy "Users can update their Luna chats"
    on public.luna_chats for update to authenticated
    using (auth.uid() = user_id)
    with check (auth.uid() = user_id);

drop policy if exists "Users can delete their Luna chats" on public.luna_chats;
create policy "Users can delete their Luna chats"
    on public.luna_chats for delete to authenticated
    using (auth.uid() = user_id);

grant select, insert, update, delete on public.luna_chats to authenticated;
