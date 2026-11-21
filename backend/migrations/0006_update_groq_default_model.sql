-- Groq retired llama-3.3-70b-versatile for standard accounts.
-- Use the currently supported GPT OSS 120B model as the default.
UPDATE public.luna_provider_settings
SET model = 'openai/gpt-oss-120b', updated_at = now()
WHERE provider = 'groq' AND model = 'llama-3.3-70b-versatile';

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
    CASE WHEN p_provider = 'groq' THEN 'openai/gpt-oss-120b' ELSE 'gemini-2.5-flash' END, v_id)
  ON CONFLICT (user_id, provider) DO UPDATE
  SET vault_secret_id = EXCLUDED.vault_secret_id, updated_at = now();
  RETURN v_id;
END;
$$;
