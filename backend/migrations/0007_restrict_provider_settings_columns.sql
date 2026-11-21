-- Do not expose Vault secret UUID references to browser sessions.
REVOKE SELECT ON TABLE public.luna_provider_settings FROM authenticated;
GRANT SELECT (provider, model, updated_at) ON TABLE public.luna_provider_settings TO authenticated;
