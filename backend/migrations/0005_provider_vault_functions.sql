ALTER TABLE public.luna_provider_settings DROP CONSTRAINT IF EXISTS luna_provider_settings_pkey;
ALTER TABLE public.luna_provider_settings ADD CONSTRAINT luna_provider_settings_pkey PRIMARY KEY (user_id, provider);

CREATE OR REPLACE FUNCTION public.luna_vault_set_secret(p_user_id uuid, p_provider text, p_secret text)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public, vault AS $$
DECLARE v_id uuid; v_name text;
BEGIN
  IF p_provider NOT IN ('groq', 'gemini') OR length(p_secret) < 8 OR length(p_secret) > 4096 THEN
    RAISE EXCEPTION 'Invalid provider or secret';
  END IF;
  v_name := 'luna_' || replace(p_user_id::text, '-', '') || '_' || p_provider;
  SELECT vault_secret_id INTO v_id FROM public.luna_provider_settings
    WHERE user_id = p_user_id AND provider = p_provider FOR UPDATE;
  IF v_id IS NULL THEN
    v_id := vault.create_secret(p_secret, v_name, 'Luna AI provider key');
  ELSE
    PERFORM vault.update_secret(v_id, p_secret, v_name, 'Luna AI provider key');
  END IF;
  INSERT INTO public.luna_provider_settings(user_id, provider, model, vault_secret_id)
  VALUES (p_user_id, p_provider,
    CASE WHEN p_provider = 'groq' THEN 'llama-3.3-70b-versatile' ELSE 'gemini-2.5-flash' END, v_id)
  ON CONFLICT (user_id, provider) DO UPDATE
  SET vault_secret_id = EXCLUDED.vault_secret_id, updated_at = now();
  RETURN v_id;
END;
$$;

CREATE OR REPLACE FUNCTION public.luna_vault_get_secret(p_user_id uuid, p_provider text)
RETURNS text LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, public, vault AS $$
  SELECT ds.decrypted_secret FROM public.luna_provider_settings ps
  JOIN vault.decrypted_secrets ds ON ds.id = ps.vault_secret_id
  WHERE ps.user_id = p_user_id AND ps.provider = p_provider LIMIT 1;
$$;

CREATE OR REPLACE FUNCTION public.luna_vault_delete_secret(p_user_id uuid, p_provider text)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public, vault AS $$
DECLARE v_id uuid;
BEGIN
  DELETE FROM public.luna_provider_settings WHERE user_id = p_user_id AND provider = p_provider
    RETURNING vault_secret_id INTO v_id;
  IF v_id IS NOT NULL THEN DELETE FROM vault.secrets WHERE id = v_id; END IF;
  RETURN true;
END;
$$;

REVOKE ALL ON FUNCTION public.luna_vault_set_secret(uuid, text, text) FROM PUBLIC, anon, authenticated;
REVOKE ALL ON FUNCTION public.luna_vault_get_secret(uuid, text) FROM PUBLIC, anon, authenticated;
REVOKE ALL ON FUNCTION public.luna_vault_delete_secret(uuid, text) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.luna_vault_set_secret(uuid, text, text) TO service_role;
GRANT EXECUTE ON FUNCTION public.luna_vault_get_secret(uuid, text) TO service_role;
GRANT EXECUTE ON FUNCTION public.luna_vault_delete_secret(uuid, text) TO service_role;
