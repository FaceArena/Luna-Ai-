import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "npm:@supabase/supabase-js@2.56.0";

const headers = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, apikey, content-type",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Content-Type": "application/json",
};
function reply(status: number, data: unknown) {
  return new Response(JSON.stringify(data), { status, headers });
}

Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") return new Response("ok", { headers });
  const url = Deno.env.get("SUPABASE_URL")!;
  const anon = Deno.env.get("SUPABASE_ANON_KEY")!;
  const service = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
  const token = (req.headers.get("Authorization") ?? "").replace(/^Bearer\s+/i, "");
  if (!token) return reply(401, { error: "Authentication required" });

  const userClient = createClient(url, anon, {
    auth: { persistSession: false, autoRefreshToken: false },
    global: { headers: { Authorization: `Bearer ${token}` } },
  });
  const { data: authData, error: authError } = await userClient.auth.getUser(token);
  if (authError || !authData.user) return reply(401, { error: "Invalid session" });
  const userId = authData.user.id;
  const admin = createClient(url, service, {
    auth: { persistSession: false, autoRefreshToken: false },
  });

  try {
    if (req.method === "GET") {
      const { data, error } = await admin.from("luna_provider_settings")
        .select("provider,model,vault_secret_id,updated_at").eq("user_id", userId)
        .order("updated_at", { ascending: false });
      if (error) throw error;
      const rows = data ?? [];
      return reply(200, {
        activeProvider: rows[0]?.provider ?? null,
        providers: rows.map((row) => ({
          provider: row.provider, model: row.model, configured: Boolean(row.vault_secret_id),
        })),
      });
    }
    if (req.method !== "POST") return reply(405, { error: "Method not allowed" });
    const body = await req.json();
    const provider = body.provider;
    if (!["groq", "gemini"].includes(provider)) return reply(400, { error: "Unsupported provider" });

    if (body.action === "select") {
      const model = typeof body.model === "string" ? body.model.trim() : "";
      const allowed = provider === "groq"
        ? ["openai/gpt-oss-120b", "openai/gpt-oss-20b"]
        : ["gemini-3.8-flash", "gemini-3.5-flash-lite"];
      if (!allowed.includes(model)) return reply(400, { error: "Unsupported model" });
      const { data: existing, error: readError } = await admin.from("luna_provider_settings")
        .select("user_id").eq("user_id", userId).eq("provider", provider).maybeSingle();
      if (readError) throw readError;
      const write = existing
        ? await admin.from("luna_provider_settings").update({ model, updated_at: new Date().toISOString() })
            .eq("user_id", userId).eq("provider", provider)
        : await admin.from("luna_provider_settings").insert({ user_id: userId, provider, model });
      if (write.error) throw write.error;
      return reply(200, { ok: true });
    }

    if (body.action === "set_key") {
      const key = typeof body.key === "string" ? body.key.trim() : "";
      if (key.length < 8 || key.length > 4096) return reply(400, { error: "Invalid API key" });
      const model = String(body.model ?? "").trim();
      const allowed = provider === "groq"
        ? ["openai/gpt-oss-120b", "openai/gpt-oss-20b"]
        : ["gemini-3.8-flash", "gemini-3.5-flash-lite"];
      if (!allowed.includes(model)) return reply(400, { error: "Unsupported model" });
      const { data: existing, error: readError } = await admin.from("luna_provider_settings")
        .select("user_id").eq("user_id", userId).eq("provider", provider).maybeSingle();
      if (readError) throw readError;
      const write = existing
        ? await admin.from("luna_provider_settings").update({ model, updated_at: new Date().toISOString() })
            .eq("user_id", userId).eq("provider", provider)
        : await admin.from("luna_provider_settings").insert({ user_id: userId, provider, model });
      if (write.error) throw write.error;
      const { error } = await admin.rpc("luna_vault_set_secret", {
        p_user_id: userId, p_provider: provider, p_secret: key,
      });
      if (error) throw error;
      return reply(200, { ok: true });
    }

    if (body.action === "delete_key") {
      const { error } = await admin.rpc("luna_vault_delete_secret", {
        p_user_id: userId, p_provider: provider,
      });
      if (error) throw error;
      return reply(200, { ok: true });
    }

    if (body.action === "chat") {
      const { data: rows, error } = await admin.from("luna_provider_settings")
        .select("provider,model,vault_secret_id,updated_at").eq("user_id", userId)
        .order("updated_at", { ascending: false }).limit(1);
      if (error) throw error;
      const setting = rows?.[0];
      if (!setting || !setting.vault_secret_id) return reply(503, { error: "No API key configured for the selected provider" });
      const { data: apiKey, error: keyError } = await admin.rpc("luna_vault_get_secret", {
        p_user_id: userId, p_provider: setting.provider,
      });
      if (keyError || !apiKey) return reply(503, { error: "Provider key unavailable" });

      const messages = Array.isArray(body.msgs) ? body.msgs : [];
      if (!messages.length || messages.length > 20 ||
          messages.some((m: any) => !["user", "assistant"].includes(m.role) ||
            typeof m.content !== "string" || m.content.length > 16000)) {
        return reply(400, { error: "Invalid chat messages" });
      }
      const system = typeof body.system === "string" ? body.system.slice(0, 8000) : "";
      let upstream: Response;
      if (setting.provider === "groq") {
        const content = [];
        if (system) content.push({ role: "system", content: system });
        content.push(...messages);
        upstream = await fetch("https://api.groq.com/openai/v1/chat/completions", {
          method: "POST",
          headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
          body: JSON.stringify({ model: setting.model, messages: content, stream: false }),
        });
      } else {
        const contents = messages.map((m: any) => ({
          role: m.role === "assistant" ? "model" : "user", parts: [{ text: m.content }],
        }));
        upstream = await fetch(
          `https://generativelanguage.googleapis.com/v1beta/models/${setting.model}:generateContent`,
          { method: "POST", headers: { "x-goog-api-key": apiKey, "Content-Type": "application/json" },
            body: JSON.stringify({ contents, ...(system ? { systemInstruction: { parts: [{ text: system }] } } : {}) }) },
        );
      }
      if (!upstream.ok) {
        if (upstream.status === 401 || upstream.status === 403) return reply(422, { error: "Provider rejected API key" });
        if (upstream.status === 429) return reply(429, { error: "Provider rate limit reached" });
        return reply(502, { error: "AI provider request failed" });
      }
      const result = await upstream.json();
      const text = setting.provider === "groq"
        ? result?.choices?.[0]?.message?.content
        : result?.candidates?.[0]?.content?.parts?.[0]?.text;
      if (typeof text !== "string") return reply(502, { error: "Invalid provider response" });
      return new Response(text, { status: 200, headers: { ...headers, "Content-Type": "text/plain; charset=utf-8" } });
    }
    return reply(400, { error: "Unknown action" });
  } catch (error) {
    console.error("luna-ai function error", error instanceof Error ? error.message : "unknown");
    return reply(500, { error: "Internal service error" });
  }
});