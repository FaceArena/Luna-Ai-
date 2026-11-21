create extension if not exists vector with schema extensions;

create table public.luna_knowledge_embeddings (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  source_kind text not null check (source_kind in ('memory', 'lorebook')),
  source_id text not null,
  title text not null default '',
  content text not null check (char_length(content) between 1 and 4000),
  embedding extensions.vector(384) not null,
  updated_at timestamptz not null default now(),
  unique (user_id, source_kind, source_id)
);

create index luna_knowledge_embeddings_owner_kind_idx
  on public.luna_knowledge_embeddings (user_id, source_kind);

create index luna_knowledge_embeddings_vector_idx
  on public.luna_knowledge_embeddings
  using hnsw (embedding extensions.vector_cosine_ops);

alter table public.luna_knowledge_embeddings enable row level security;

create policy "Users can read their own knowledge embeddings"
  on public.luna_knowledge_embeddings
  for select
  to authenticated
  using ((select auth.uid()) = user_id);

grant select on public.luna_knowledge_embeddings to authenticated;
grant all on public.luna_knowledge_embeddings to service_role;

create or replace function public.match_luna_knowledge(
  query_embedding extensions.vector(384),
  match_count integer default 6,
  min_similarity real default 0.45,
  source_filter text default null
)
returns table (
  source_kind text,
  source_id text,
  title text,
  content text,
  similarity real
)
language sql
stable
security invoker
set search_path = ''
as $$
  select
    k.source_kind,
    k.source_id,
    k.title,
    k.content,
    (1 - (k.embedding OPERATOR(extensions.<=>) query_embedding))::real as similarity
  from public.luna_knowledge_embeddings as k
  where k.user_id = (select auth.uid())
    and (source_filter is null or k.source_kind = source_filter)
    and 1 - (k.embedding OPERATOR(extensions.<=>) query_embedding) >= min_similarity
  order by k.embedding OPERATOR(extensions.<=>) query_embedding
  limit least(greatest(match_count, 1), 10);
$$;

revoke all on function public.match_luna_knowledge(extensions.vector, integer, real, text) from public;
grant execute on function public.match_luna_knowledge(extensions.vector, integer, real, text) to authenticated;
