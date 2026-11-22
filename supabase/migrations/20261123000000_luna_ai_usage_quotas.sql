-- Persistent, per-user Luna AI quotas and provider token accounting.
create table if not exists public.luna_ai_usage (
  user_id uuid not null references auth.users(id) on delete cascade,
  usage_date date not null,
  request_count integer not null default 0 check (request_count >= 0),
  prompt_tokens bigint not null default 0 check (prompt_tokens >= 0),
  completion_tokens bigint not null default 0 check (completion_tokens >= 0),
  recent_requests timestamptz[] not null default '{}'::timestamptz[],
  updated_at timestamptz not null default now(),
  primary key (user_id, usage_date)
);

alter table public.luna_ai_usage enable row level security;
revoke all on table public.luna_ai_usage from public, anon, authenticated;
grant all on table public.luna_ai_usage to service_role;

create or replace function public.luna_reserve_ai_request(
  p_user_id uuid,
  p_tier text,
  p_requests_per_minute integer
)
returns jsonb
language plpgsql
security definer
set search_path = pg_catalog, public
as $$
declare
  v_day date := (now() at time zone 'UTC')::date;
  v_daily_limit integer;
  v_rpm integer := greatest(1, least(coalesce(p_requests_per_minute, 1), 60));
  v_usage public.luna_ai_usage%rowtype;
  v_recent timestamptz[];
  v_retry integer;
begin
  if p_user_id is null then
    raise exception 'user_id is required';
  end if;

  v_daily_limit := case p_tier
    when 'luna_pro' then 2000
    when 'luna' then 600
    else 100
  end;

  insert into public.luna_ai_usage (user_id, usage_date)
  values (p_user_id, v_day)
  on conflict (user_id, usage_date) do nothing;

  select * into v_usage
  from public.luna_ai_usage
  where user_id = p_user_id and usage_date = v_day
  for update;

  select coalesce(array_agg(t), '{}'::timestamptz[])
  into v_recent
  from unnest(v_usage.recent_requests) as t
  where t > now() - interval '60 seconds';

  if v_usage.request_count >= v_daily_limit then
    v_retry := greatest(1, ceil(extract(epoch from (((v_day + 1)::timestamp at time zone 'UTC') - now())))::integer);
    return jsonb_build_object(
      'allowed', false, 'reason', 'daily',
      'requests_used', v_usage.request_count,
      'daily_limit', v_daily_limit,
      'retry_after_seconds', v_retry,
      'resets_at', ((v_day + 1)::timestamp at time zone 'UTC')
    );
  end if;

  if cardinality(v_recent) >= v_rpm then
    select greatest(1, ceil(60 - extract(epoch from (now() - min(t)))))::integer
    into v_retry
    from unnest(v_recent) as t;
    return jsonb_build_object(
      'allowed', false, 'reason', 'minute',
      'requests_used', v_usage.request_count,
      'daily_limit', v_daily_limit,
      'retry_after_seconds', coalesce(v_retry, 5)
    );
  end if;

  update public.luna_ai_usage
  set request_count = request_count + 1,
      recent_requests = v_recent || now(),
      updated_at = now()
  where user_id = p_user_id and usage_date = v_day;

  return jsonb_build_object(
    'allowed', true,
    'usage_date', v_day,
    'requests_used', v_usage.request_count + 1,
    'daily_limit', v_daily_limit,
    'requests_remaining', greatest(0, v_daily_limit - v_usage.request_count - 1)
  );
end;
$$;

create or replace function public.luna_record_ai_tokens(
  p_user_id uuid,
  p_usage_date date,
  p_prompt_tokens bigint,
  p_completion_tokens bigint
)
returns void
language sql
security definer
set search_path = pg_catalog, public
as $$
  update public.luna_ai_usage
  set prompt_tokens = prompt_tokens + greatest(0, coalesce(p_prompt_tokens, 0)),
      completion_tokens = completion_tokens + greatest(0, coalesce(p_completion_tokens, 0)),
      updated_at = now()
  where user_id = p_user_id and usage_date = p_usage_date;
$$;

revoke all on function public.luna_reserve_ai_request(uuid, text, integer) from public, anon, authenticated;
revoke all on function public.luna_record_ai_tokens(uuid, date, bigint, bigint) from public, anon, authenticated;
grant execute on function public.luna_reserve_ai_request(uuid, text, integer) to service_role;
grant execute on function public.luna_record_ai_tokens(uuid, date, bigint, bigint) to service_role;
