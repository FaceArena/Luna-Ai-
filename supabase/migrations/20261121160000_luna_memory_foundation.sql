-- Durable, user-isolated roleplay memory for Luna.
-- Chat-scoped rows are deleted automatically with their owning chat.
create table if not exists public.luna_memories (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  chat_id bigint null,
  category text not null check (category in (
    'character', 'relationship', 'world', 'event',
    'preference', 'correction', 'open_thread', 'other'
  )),
  content text not null check (char_length(content) between 1 and 4000),
  source text not null default 'explicit'
    check (source in ('explicit', 'correction', 'inferred')),
  importance smallint not null default 3 check (importance between 1 and 5),
  confidence numeric(3,2) not null default 1.00 check (confidence between 0 and 1),
  is_pinned boolean not null default false,
  last_used_at timestamptz null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint luna_memories_chat_owner_fkey
    foreign key (user_id, chat_id)
    references public.luna_chats(user_id, id)
    on delete cascade,
  constraint luna_memories_scope_check
    check ((category in ('character','relationship','world','preference') and chat_id is null)
        or chat_id is not null
        or category = 'other')
);

create index if not exists luna_memories_user_scope_idx
  on public.luna_memories (user_id, category, importance desc, updated_at desc)
  where chat_id is null;

create index if not exists luna_memories_chat_scope_idx
  on public.luna_memories (user_id, chat_id, category, importance desc, updated_at desc)
  where chat_id is not null;

create table if not exists public.luna_chat_memory_state (
  user_id uuid not null references auth.users(id) on delete cascade,
  chat_id bigint not null,
  summary text not null default '' check (char_length(summary) <= 12000),
  last_scene text not null default '' check (char_length(last_scene) <= 8000),
  open_threads jsonb not null default '[]'::jsonb
    check (jsonb_typeof(open_threads) = 'array'),
  updated_at timestamptz not null default now(),
  primary key (user_id, chat_id),
  constraint luna_chat_memory_state_chat_owner_fkey
    foreign key (user_id, chat_id)
    references public.luna_chats(user_id, id)
    on delete cascade
);

alter table public.luna_memories enable row level security;
alter table public.luna_chat_memory_state enable row level security;

revoke all on table public.luna_memories from public, anon, authenticated;
revoke all on table public.luna_chat_memory_state from public, anon, authenticated;
grant select, insert, update, delete on table public.luna_memories to authenticated;
grant select, insert, update, delete on table public.luna_chat_memory_state to authenticated;

drop policy if exists "Users manage their own Luna memories" on public.luna_memories;
create policy "Users manage their own Luna memories"
  on public.luna_memories
  for all to authenticated
  using ((select auth.uid()) = user_id)
  with check ((select auth.uid()) = user_id);

drop policy if exists "Users manage their own Luna chat memory state" on public.luna_chat_memory_state;
create policy "Users manage their own Luna chat memory state"
  on public.luna_chat_memory_state
  for all to authenticated
  using ((select auth.uid()) = user_id)
  with check ((select auth.uid()) = user_id);
