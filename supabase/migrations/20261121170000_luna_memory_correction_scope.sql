-- Allow global correction records.
alter table public.luna_memories drop constraint if exists luna_memories_scope_check;
alter table public.luna_memories add constraint luna_memories_scope_check
check (
  (category in ('character','relationship','world','preference','correction') and chat_id is null)
  or chat_id is not null
  or category = 'other'
);
