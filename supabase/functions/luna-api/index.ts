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
    if (method === "POST" && path === "/api/register") {
      const body = await req.json();
      const email = typeof body.email === "string" ? body.email.trim() : "";
      const password = typeof body.pass === "string" ? body.pass : "";
      const name = typeof body.name === "string" ? body.name.trim().slice(0, 40) : "";
      const validEmail = /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
      if (!validEmail || email.length > 254 || password.length < 8 || password.length > 128 || !name)
        return text(400, "Проверьте имя, адрес почты и пароль (8–128 символов).");
      const client = createClient(base, anon, { auth: { persistSession: false, autoRefreshToken: false } });
      const { data, error } = await client.auth.signUp({ email, password, options: { data: { name } } });
      if (error) {
        const message = error.message.toLowerCase();
        const code = String((error as any).code ?? "");
        if (code === "user_already_exists" || message.includes("already registered") || message.includes("already been registered")) {
          return text(409, "Аккаунт с этой почтой уже существует. Войдите.");
        }
        if (message.includes("invalid") && message.includes("email")) {
          return text(400, "Проверьте адрес электронной почты.");
        }
        if (message.includes("password") || code.includes("password")) {
          return text(400, "Пароль не прошёл проверку безопасности. Используйте более надёжный пароль.");
        }
        return text(400, "Не удалось создать аккаунт. Проверьте данные и попробуйте ещё раз.");
      }
      if (!data.session) return json(202, { email, status: "confirmation_required" });
      return json(200, { name, email, plan: 0 }, { "Set-Cookie": sessionCookie(data.session.access_token) });
    }

    if (method === "POST" && path === "/api/verify-email") {
      const body = await req.json().catch(() => null);
      const email = typeof body?.email === "string" ? body.email.trim() : "";
      const code = typeof body?.code === "string" ? body.code.trim() : "";
      if (!/^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/.test(email) || !/^\\d{6}$/.test(code)) {
        return text(400, "Введите корректную почту и шестизначный код.");
      }
      const client = createClient(base, anon, { auth: { persistSession: false, autoRefreshToken: false } });
      const { data, error } = await client.auth.verifyOtp({ email, token: code, type: "signup" });
      if (error || !data.session || !data.user) {
        return text(400, "Код неверный или срок его действия истёк. Проверьте письмо и попробуйте ещё раз.");
      }
      const name = String(data.user.user_metadata?.name ?? "Luna user");
      const plan = Math.min(2, Number(data.user.user_metadata?.plan ?? 0) || 0);
      return json(200, { name, email: data.user.email ?? email, plan }, { "Set-Cookie": sessionCookie(data.session.access_token) });
    }

    if (method === "POST" && path === "/api/login") {
      const body = await req.json().catch(() => null);
      const email = typeof body?.email === "string" ? body.email.trim() : "";
      const password = typeof body?.pass === "string" ? body.pass : "";
      if (!email || email.length > 254 || password.length < 8 || password.length > 128)
        return text(400, "Введите корректную почту и пароль (8–128 символов).");
      const client = createClient(base, anon, { auth: { persistSession: false, autoRefreshToken: false } });
      const { data, error } = await client.auth.signInWithPassword({ email, password });
      if (error || !data.session || !data.user) return text(401, "Неверная почта или пароль.");
      const name = String(data.user.user_metadata?.name ?? "Luna user");
      const plan = Math.min(2, Number(data.user.user_metadata?.plan ?? 0) || 0);
      return json(200, { name, email: data.user.email ?? "", plan }, { "Set-Cookie": sessionCookie(data.session.access_token) });
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
      const { data: apiKey, error: keyError } = await admin.rpc("luna_get_global_secret", {
        p_name: "luna_global_groq_api_key",
      });
      if (keyError || typeof apiKey !== "string" || !apiKey) {
        return text(503, "AI-сервис временно недоступен. Попробуйте позже.");
      }
      const model = "openai/gpt-oss-120b";
      const system = [
        `You are ${String(body.name ?? "Luna").slice(0, 80)}.`,
        `Role: ${String(body.who ?? "").slice(0, 1000)}.`,
        `Rules: ${String(body.rules ?? "").slice(0, 4000)}.`,
        `Response length: ${String(body.len ?? "normal").slice(0, 40)}.`,
        `User name: ${String(body.you ?? "").slice(0, 80)}.`,
      ].join(" ");
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