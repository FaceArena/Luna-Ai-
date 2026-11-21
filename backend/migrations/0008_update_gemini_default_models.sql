-- Move Gemini defaults to current stable models available to new projects.
UPDATE public.luna_provider_settings
SET model = 'gemini-3.8-flash', updated_at = now()
WHERE provider = 'gemini' AND model IN ('gemini-2.5-flash', 'gemini-2.5-pro');

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
    CASE WHEN p_provider = 'groq' THEN 'openai/gpt-oss-120b' ELSE 'gemini-3.8-flash' END, v_id)
  ON CONFLICT (user_id, provider) DO UPDATE
  SET vault_secret_id = EXCLUDED.vault_secret_id, updated_at = now();
  RETURN v_id;
END;
$$;
