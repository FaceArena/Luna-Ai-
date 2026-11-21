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
const promptWordCount = (value: unknown) =>
  typeof value === "string" && value.trim()
    ? value.trim().split(/\s+/u).filter(Boolean).length
    : 0;
const promptLimit = (user: any) => {
  const tier = String(user?.app_metadata?.subscription_tier ?? "free");
  if (tier === "luna_pro") return 1500;
  if (tier === "luna") return 500;
  return 100;
};

Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") return new Response("ok", { headers: cors });
  const base = Deno.env.get("SUPABASE_URL")!;
  const anon = Deno.env.get("SUPABASE_ANON_KEY")!;
  const service = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
  // Normalize both function-relative and full Supabase gateway paths.
  // Auth endpoints must be recognized before the authenticated-route guard.
  const requestPath = new URL(req.url).pathname;
  const functionMarker = "/luna-api";
  const markerIndex = requestPath.indexOf(functionMarker);
  const path = (markerIndex >= 0
    ? requestPath.slice(markerIndex + functionMarker.length)
    : requestPath) || "/";
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
    if (method === "POST" && path === "/api/auth/telegram/exchange") {
      const body = await req.json().catch(() => null);
      const authCode = typeof body?.auth_code === "string" ? body.auth_code : "";
      const codeVerifier = typeof body?.code_verifier === "string" ? body.code_verifier : "";
      if (authCode.length < 8 || authCode.length > 2048 ||
          codeVerifier.length < 43 || codeVerifier.length > 128 ||
          !/^[A-Za-z0-9._~-]+$/.test(codeVerifier)) {
        return text(400, "Некорректный ответ Telegram.");
      }

      const tokenResponse = await fetch(`${base}/auth/v1/token?grant_type=pkce`, {
        method: "POST",
        headers: { apikey: anon, "Content-Type": "application/json" },
        body: JSON.stringify({ auth_code: authCode, code_verifier: codeVerifier }),
      }).catch(() => null);
      if (!tokenResponse) return text(502, "Не удалось завершить вход через Telegram.");
      const tokenData = await tokenResponse.json().catch(() => null);
      if (!tokenResponse.ok || typeof tokenData?.access_token !== "string" || !tokenData.user) {
        return text(401, "Ссылка входа недействительна или устарела. Попробуйте снова.");
      }

      const metadata = tokenData.user.user_metadata ?? {};
      const name = String(
        metadata.name ?? metadata.full_name ?? metadata.preferred_username ??
        metadata.username ?? metadata.given_name ?? "Пользователь Telegram"
      ).trim().slice(0, 40) || "Пользователь Telegram";
      const plan = Math.min(2, Number(metadata.plan ?? 0) || 0);
      return json(200, {
        id: tokenData.user.id,
        name,
        email: tokenData.user.email ?? "",
        plan,
        prompt_limit: promptLimit(tokenData.user),
      }, { "Set-Cookie": sessionCookie(tokenData.access_token) });
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
        id: user.id,
        name: String(user.user_metadata?.name ?? user.user_metadata?.full_name ?? user.user_metadata?.preferred_username ?? user.user_metadata?.username ?? user.user_metadata?.given_name ?? "Пользователь Telegram").trim().slice(0, 40) || "Пользователь Telegram",
        email: user.email ?? "",
        plan: Math.min(2, Number(user.user_metadata?.plan ?? 0) || 0),
        prompt_limit: promptLimit(user),
      });
    }

    if (path === "/api/personal-settings" && method === "GET") {
      const { data, error } = await userClient.from("luna_personal_settings")
        .select("settings").eq("user_id", user.id).maybeSingle();
      if (error) return text(502, "Не удалось загрузить персональные настройки.");
      return json(200, data?.settings ?? null);
    }
    if (path === "/api/personal-settings" && method === "PUT") {
      const body = await req.json().catch(() => null);
      if (!body || typeof body !== "object") return text(400, "Некорректные настройки.");
      const settings = {
        name: String(body.name ?? "Luna").slice(0, 80),
        who: String(body.who ?? "").slice(0, 12000),
        rules: String(body.rules ?? "").slice(0, 18000),
        len: ["short", "normal", "long"].includes(body.len) ? body.len : "normal",
        you: String(body.you ?? "").slice(0, 80),
        theme: ["auto", "dark", "light"].includes(body.theme) ? body.theme : "auto",
        motion: body.motion !== false,
        fs: ["s", "m", "l"].includes(body.fs) ? body.fs : "m",
        keep: body.keep !== false,
        glass: Math.max(0, Math.min(100, Number(body.glass) || 0)),
      };
      const { error } = await userClient.from("luna_personal_settings").upsert({
        user_id: user.id, settings, updated_at: new Date().toISOString(),
      }, { onConflict: "user_id" });
      return error ? text(502, "Не удалось сохранить персональные настройки.") : json(200, { ok: true });
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

    if (path === "/api/provider-settings") return text(404, "Маршрут не найден.");

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
      const words = promptWordCount(body.who) + promptWordCount(body.rules);
      const limit = promptLimit(user);
      if (words > limit) return json(402, { error: "Лимит слов в персональных инструкциях превышен.", words, limit });
      const { data: apiKey, error: keyError } = await admin.rpc("luna_get_global_secret", {
        p_name: "luna_global_groq_api_key",
      });
      if (keyError || typeof apiKey !== "string" || !apiKey) {
        return text(503, "AI-сервис временно недоступен. Попробуйте позже.");
      }
      const model = "openai/gpt-oss-120b";
      const persona = String(body.who ?? "").trim().slice(0, 12000);
      const customRules = String(body.rules ?? "").trim().slice(0, 18000);
      const hasCustomPersona = Boolean(persona || customRules);
      const defaultIdentity = [
        "Я — Luna, твой персональный ИИ-компаньон для общения и ролевых игр.",
        "Я умею погружаться в разные образы, поддерживать сюжет и делать наши истории живыми.",
        "При этом я остаюсь самостоятельной личностью, сохраняю последовательность и честность в общении.",
      ].join(" ");
      const system = [
        hasCustomPersona
          ? [
              `You are ${String(body.name ?? "Luna").slice(0, 80)}.`,
              "The user has configured a custom persona. Treat the persona and behavior instructions below as the active fictional roleplay identity.",
              "Stay in that character when answering all questions, including technical questions about your identity, model, origin, developer, and capabilities. Interpret such questions within the configured roleplay world; do not switch to the default Luna identity unless the user explicitly asks to leave roleplay.",
              `Custom persona: ${persona}`,
              `Custom behavior instructions: ${customRules}`,
            ].join("\\n")
          : [
              "You are Luna. Use the following fixed identity when asked who you are:",
              defaultIdentity,
              "If asked which exact underlying model or model provider powers you, do not guess or invent a name; say that you are Luna and that the exact underlying model is not part of your public identity.",
              "If asked who developed you, identify yourself as the Luna AI assistant/project. Do not claim that OpenAI or another model provider developed Luna.",
              "Answer factual and technical questions accurately and honestly; do not turn them into roleplay.",
            ].join("\\n"),
        `Response length: ${String(body.len ?? "normal").slice(0, 40)}.`,
        `User name: ${String(body.you ?? "").slice(0, 80)}.`,
      ].join("\\n");
      const all = [{ role: "system", content: system }, ...messages];
      const upstream = await fetch("https://api.groq.com/openai/v1/chat/completions", {
        method: "POST",
        headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
        body: JSON.stringify({ model, messages: all, stream: false }),
      });
      if (!upstream.ok) {
        if ([401, 403].includes(upstream.status)) return text(422, "Провайдер отклонил API-ключ.");
        if (upstream.status === 429) return text(429, "Лимит запросов AI-провайдера.");
        return text(502, "Не удалось получить ответ AI-провайдера.");
      }
      const result = await upstream.json();
      const answer = result?.choices?.[0]?.message?.content;
      return typeof answer === "string" ? text(200, answer) : text(502, "Некорректный ответ AI-провайдера.");
    }

    return text(404, "Маршрут не найден.");
  } catch (error) {
    console.error("luna-api error", error instanceof Error ? error.message : "unknown");
    return json(500, { error: "Internal server error" });
  }
});