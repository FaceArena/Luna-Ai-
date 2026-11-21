create or replace function public.luna_get_global_secret(p_name text)
returns text
language sql
security definer
set search_path = pg_catalog, vault
as $function$
  select decrypted_secret
  from vault.decrypted_secrets
  where name = p_name
    and p_name in ('luna_global_groq_api_key', 'luna_global_gemini_api_key')
  limit 1;
$function$;

revoke all on function public.luna_get_global_secret(text) from public;
revoke all on function public.luna_get_global_secret(text) from anon, authenticated;
grant execute on function public.luna_get_global_secret(text) to service_role;
