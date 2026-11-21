import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "npm:@supabase/supabase-js@2.56.0";

const cors = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, apikey, content-type",
  "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS",
};
const json = (status: number, value: unknown, extra: HeadersInit = {}) =>
  new Response(JSON.stringify(value), { status, headers: { ...cors, "Content-Type": "application/json", ...extra } });
const text = (status: number, value: string, extra: HeadersInit = {}) =>
  new Response(value, { status, headers: { ...cors, "Content-Type": "text/plain; charset=utf-8", ...extra } });

function cookieToken(req: Request) {
  const cookie = req.headers.get("cookie") ?? "";
  const pair = cookie.split(";").map((part) => part.trim()).find((part) => part.startsWith("luna_session="));
  return pair ? decodeURIComponent(pair.slice("luna_session=".length)) : "";
}
function sessionCookie(token: string) {
  return `luna_session=${token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=3600`;
}
const clearCookie = "luna_session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0";
const chatRateLimits = new Map<string, number[]>();

Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") return new Response("ok", { headers: cors });
  const base = Deno.env.get("SUPABASE_URL")!;
  const anon = Deno.env.get("SUPABASE_ANON_KEY")!;
  const service = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
  const path = new URL(req.url).pathname.replace(/^\/functions\/v1\/luna-api/, "") || "/";
  const method = req.method;
  const admin = createClient(base, service, { auth: { persistSession: false, autoRefreshToken: false } });

  if (method === "GET" && path === "/health") return json(200, { service: "luna-api", status: "ok" });

  let token = cookieToken(req);
  const authorization = req.headers.get("Authorization") ?? "";
  if (authorization.startsWith("Bearer ")) token = authorization.slice(7);
  const userClient = token ? createClient(base, anon, {
    auth: { persistSession: false, autoRefreshToken: false },
    global: { headers: { Authorization: `Bearer ${token}` } },
  }) : null;

  try {
    if (method === "POST" && path === "/api/register") {
      const body = await req.json();
      const email = typeof body.email === "string" ? body.email.trim() : "";
      const password = typeof body.pass === "string" ? body.pass : "";
      const name = typeof body.name === "string" ? body.name.trim().slice(0, 40) : "";
      if (!email || email.length > 254 || password.length < 8 || password.length > 128 || !name)
        return text(400, "Проверьте имя, почту и пароль (8–128 символов).");
      const client = createClient(base, anon, { auth: { persistSession: false, autoRefreshToken: false } });
      const { data, error } = await client.auth.signUp({ email, password, options: { data: { name } } });
      if (error) return text(400, error.message);
      if (!data.session) return text(409, "Аккаунт создан. Подтвердите email, затем войдите.");
      return json(200, { name, email }, { "Set-Cookie": sessionCookie(data.session.access_token) });
    }

    if (method === "POST" && path === "/api/login") {
      const body = await req.json();
      const client = createClient(base, anon, { auth: { persistSession: false, autoRefreshToken: false } });
      const { data, error } = await client.auth.signInWithPassword({
        email: String(body.email ?? "").trim(), password: String(body.pass ?? ""),
      });
      if (error || !data.session || !data.user) return text(401, "Неверная почта или пароль.");
      const name = String(data.user.user_metadata?.name ?? "Luna user");
      return json(200, { name, email: data.user.email ?? "" }, { "Set-Cookie": sessionCookie(data.session.access_token) });
    }

    if (method === "POST" && path === "/api/logout") {
      if (token) {
        await fetch(`${base}/auth/v1/logout`, {
          method: "POST",
          headers: { apikey: anon, Authorization: `Bearer ${token}` },
        }).catch(() => undefined);
      }
      return new Response(null, { status: 204, headers: { ...cors, "Set-Cookie": clearCookie } });
    }

    if (!userClient || !token) return text(401, "Требуется вход.");
    const { data: authData, error: authError } = await userClient.auth.getUser(token);
    if (authError || !authData.user) return text(401, "Сессия закончилась. Войдите снова.");
    const user = authData.user;

    if (method === "GET" && path === "/api/me") {
      return json(200, {
        name: String(user.user_metadata?.name ?? "Luna user"),
        email: user.email ?? "",
        plan: Math.min(2, Number(user.user_metadata?.plan ?? 0) || 0),
      });
    }

    if (path === "/api/chats" && method === "GET") {
      const { data, error } = await userClient.from("luna_chats")
        .select("id,title,msgs").order("updated_at", { ascending: false });
      if (error) return text(502, "Ошибка хранилища истории.");
      return json(200, data ?? []);
    }
    if (path === "/api/chats" && method === "DELETE") {
      const { error } = await userClient.from("luna_chats").delete().eq("user_id", user.id);
      return error ? text(502, "Ошибка очистки истории.") : new Response(null, { status: 204, headers: cors });
    }
    const chatMatch = path.match(/^\/api\/chats\/(-?\d+)$/);
    if (chatMatch && method === "PUT") {
      const id = Number(chatMatch[1]);
      const body = await req.json();
      const msgs = Array.isArray(body.msgs) ? body.msgs : [];
      const title = typeof body.title === "string" ? body.title : "";
      if (!Number.isSafeInteger(id) || body.id !== id || msgs.length > 500 || title.length > 160 ||
          msgs.some((m: any) => typeof m.content !== "string" || m.content.length > 32000))
        return text(400, "Некорректные данные чата.");
      const { error } = await userClient.from("luna_chats").upsert({
        user_id: user.id, id, title, msgs, updated_at: new Date().toISOString(),
      }, { onConflict: "user_id,id" });
      return error ? text(502, "Ошибка сохранения чата.") : new Response(null, { status: 204, headers: cors });
    }
    if (chatMatch && method === "DELETE") {
      const { error } = await userClient.from("luna_chats").delete().eq("id", Number(chatMatch[1]));
      return error ? text(502, "Ошибка удаления чата.") : new Response(null, { status: 204, headers: cors });
    }

    if (path === "/api/plan" && method === "PUT") {
      const body = await req.json();
      const plan = Math.max(0, Math.min(2, Number(body.plan) || 0));
      const { error } = await userClient.auth.updateUser({ data: { plan } });
      return error ? text(502, "Не удалось сохранить выбор тарифа.") : new Response(null, { status: 204, headers: cors });
    }

    if (path === "/api/provider-settings" && method === "GET") {
      const { data, error } = await admin.from("luna_provider_settings")
        .select("provider,model,vault_secret_id,updated_at").eq("user_id", user.id)
        .order("updated_at", { ascending: false });
      if (error) return json(500, { error: "Unable to load provider settings" });
      const rows = data ?? [];
      return json(200, {
        activeProvider: rows[0]?.provider ?? null,
        providers: rows.map((r) => ({ provider: r.provider, model: r.model, configured: Boolean(r.vault_secret_id) })),
      });
    }

    if (path === "/api/provider-settings" && method === "POST") {
      const body = await req.json();
      const provider = body.provider;
      if (!["groq", "gemini"].includes(provider)) return json(400, { error: "Unsupported provider" });
      if (body.action === "select") {
        const model = String(body.model ?? "").trim();
        const allowed = provider === "groq"
          ? ["openai/gpt-oss-120b", "openai/gpt-oss-20b"]
          : ["gemini-3.8-flash", "gemini-3.5-flash-lite"];
        if (!allowed.includes(model)) return json(400, { error: "Unsupported model" });
        const { data: existing, error: readError } = await admin.from("luna_provider_settings")
          .select("user_id").eq("user_id", user.id).eq("provider", provider).maybeSingle();
        if (readError) throw readError;
        const write = existing
          ? await admin.from("luna_provider_settings").update({ model, updated_at: new Date().toISOString() })
              .eq("user_id", user.id).eq("provider", provider)
          : await admin.from("luna_provider_settings").insert({ user_id: user.id, provider, model });
        if (write.error) throw write.error;
        return json(200, { ok: true });
      }
      if (body.action === "set_key") {
        const key = typeof body.key === "string" ? body.key.trim() : "";
        if (key.length < 8 || key.length > 4096) return json(400, { error: "Invalid API key" });
        const model = String(body.model ?? "").trim();
        const allowed = provider === "groq"
          ? ["openai/gpt-oss-120b", "openai/gpt-oss-20b"]
          : ["gemini-3.8-flash", "gemini-3.5-flash-lite"];
        if (!allowed.includes(model)) return json(400, { error: "Unsupported model" });
        const { data: existing, error: readError } = await admin.from("luna_provider_settings")
          .select("user_id").eq("user_id", user.id).eq("provider", provider).maybeSingle();
        if (readError) throw readError;
        const write = existing
          ? await admin.from("luna_provider_settings").update({ model, updated_at: new Date().toISOString() })
              .eq("user_id", user.id).eq("provider", provider)
          : await admin.from("luna_provider_settings").insert({ user_id: user.id, provider, model });
        if (write.error) throw write.error;
        const { error } = await admin.rpc("luna_vault_set_secret", {
          p_user_id: user.id, p_provider: provider, p_secret: key,
        });
        if (error) throw error;
        return json(200, { ok: true });
      }
      if (body.action === "delete_key") {
        const { error } = await admin.rpc("luna_vault_delete_secret", { p_user_id: user.id, p_provider: provider });
        if (error) throw error;
        return json(200, { ok: true });
      }
      return json(400, { error: "Unknown provider settings action" });
    }

    if (path === "/api/chat" && method === "POST") {
      const now = Date.now();
      const recent = (chatRateLimits.get(user.id) ?? []).filter((at) => now - at < 60_000);
      if (recent.length >= 12) return text(429, "Слишком много сообщений. Подождите минуту.");
      recent.push(now);
      chatRateLimits.set(user.id, recent);
      const body = await req.json();
      const messages = Array.isArray(body.msgs) ? body.msgs : [];
      if (!messages.length || messages.length > 20 || messages.some((m: any) =>
        !["user", "assistant"].includes(m.role) || typeof m.content !== "string" || m.content.length > 16000))
        return text(400, "Слишком длинный запрос или некорректная история.");
      const { data: settings, error } = await admin.from("luna_provider_settings")
        .select("provider,model,vault_secret_id,updated_at").eq("user_id", user.id)
        .order("updated_at", { ascending: false }).limit(1);
      if (error) throw error;
      const setting = settings?.[0];
      if (!setting || !setting.vault_secret_id) return text(503, "Для выбранного AI-провайдера не сохранён API-ключ.");
      const { data: apiKey, error: keyError } = await admin.rpc("luna_vault_get_secret", {
        p_user_id: user.id, p_provider: setting.provider,
      });
      if (keyError || !apiKey) return text(503, "Не удалось получить ключ AI-провайдера.");
      const system = [
        `You are ${String(body.name ?? "Luna").slice(0, 80)}.`,
        `Role: ${String(body.who ?? "").slice(0, 1000)}.`,
        `Rules: ${String(body.rules ?? "").slice(0, 4000)}.`,
        `Response length: ${String(body.len ?? "normal").slice(0, 40)}.`,
        `User name: ${String(body.you ?? "").slice(0, 80)}.`,
      ].join(" ");
      let upstream: Response;
      if (setting.provider === "groq") {
        const all = [{ role: "system", content: system }, ...messages];
        upstream = await fetch("https://api.groq.com/openai/v1/chat/completions", {
          method: "POST", headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
          body: JSON.stringify({ model: setting.model, messages: all, stream: false }),
        });
      } else {
        const contents = messages.map((m: any) => ({
          role: m.role === "assistant" ? "model" : "user", parts: [{ text: m.content }],
        }));
        upstream = await fetch(
          `https://generativelanguage.googleapis.com/v1beta/models/${setting.model}:generateContent`,
          { method: "POST", headers: { "x-goog-api-key": apiKey, "Content-Type": "application/json" },
            body: JSON.stringify({ contents, systemInstruction: { parts: [{ text: system }] } }) },
        );
      }
      if (!upstream.ok) {
        if ([401, 403].includes(upstream.status)) return text(422, "Провайдер отклонил API-ключ.");
        if (upstream.status === 429) return text(429, "Лимит запросов AI-провайдера.");
        return text(502, "Не удалось получить ответ AI-провайдера.");
      }
      const result = await upstream.json();
      const answer = setting.provider === "groq"
        ? result?.choices?.[0]?.message?.content
        : result?.candidates?.[0]?.content?.parts?.[0]?.text;
      return typeof answer === "string" ? text(200, answer) : text(502, "Некорректный ответ AI-провайдера.");
    }

    return text(404, "Маршрут не найден.");
  } catch (error) {
    console.error("luna-api error", error instanceof Error ? error.message : "unknown");
    return json(500, { error: "Internal server error" });
  }
});