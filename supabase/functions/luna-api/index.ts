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

const MEMORY_CATEGORIES = new Set(["character", "relationship", "world", "event", "preference", "correction", "open_thread", "other"]);
function memoryText(value: unknown, max = 1800) {
  return typeof value === "string" ? value.trim().slice(0, max) : "";
}
const INTERNAL_PROVIDER_PATTERNS = [
  /\bgroq\b/i,
  /api\.groq\.com/i,
  /openai\/gpt-oss-120b/i,
  /\bgpt-oss-120b\b/i,
  /\bopenai\b/i,
  /\bsupabase\b/i,
  /\bcloudflare\b/i,
];
function protectPublicIdentity(answer: string) {
  if (!INTERNAL_PROVIDER_PATTERNS.some((pattern) => pattern.test(answer))) return answer;
  return "Я — Luna, твой персональный ИИ-компаньон. Моя внутренняя техническая реализация не является частью моей публичной личности. Давай продолжим наш разговор.";
}
async function saveRoleplayMemory(client: any, userId: string, chatId: number, keepChat: boolean, model: string, apiKey: string, messages: any[], answer: string) {
  const extractionMessages = [
    { role: "system", content: [
      'Extract durable roleplay memory. Return ONLY JSON: {"memories":[{"category":"character|relationship|world|event|preference|correction|open_thread|other","scope":"global|chat","content":"...","importance":1,"confidence":0.9,"source":"explicit|correction"}],"summary":"...","last_scene":"...","open_threads":["..."]}.',
      "Save only facts explicitly stated or clearly confirmed by the USER. Do not save assistant guesses, temporary banter, or unconfirmed assumptions.",
      "Corrections supersede older facts. Global scope is only for stable character traits, relationship facts, world rules and persistent preferences. Scene events and plot continuity use chat scope.",
      "Never save passwords, API keys, authentication codes, payment details, precise addresses, sensitive health information, or private identifying data.",
      "At most 5 concise atomic memories, each under 500 characters. If nothing durable is new, return an empty memories array.",
      "Summary and last_scene describe THIS chat only. Never invent missing details."
    ].join("\n") },
    { role: "user", content: JSON.stringify({ messages: messages.slice(-10), latest_assistant_reply: answer.slice(0, 5000) }) }
  ];
  try {
    const response = await fetch("https://api.groq.com/openai/v1/chat/completions", {
      method: "POST",
      headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
      body: JSON.stringify({ model, messages: extractionMessages, temperature: 0.1, max_tokens: 1400, response_format: { type: "json_object" } })
    });
    if (!response.ok) return;
    const payload = await response.json();
    const raw = payload?.choices?.[0]?.message?.content;
    if (typeof raw !== "string") return;
    const parsed = JSON.parse(raw);
    const candidates = Array.isArray(parsed.memories) ? parsed.memories.slice(0, 5) : [];
    for (const item of candidates) {
      const category = String(item?.category ?? "");
      const content = memoryText(item?.content, 500);
      const scope = item?.scope === "global" || !keepChat ? "global" : "chat";
      if (!MEMORY_CATEGORIES.has(category) || !content) continue;
      if (scope === "global" && !["character", "relationship", "world", "preference", "correction"].includes(category)) continue;
      const chatScope = scope === "global" ? null : chatId;
      const { data: existing } = await client.from("luna_memories").select("id,content,source")
        .eq("user_id", userId).eq("category", category).is("chat_id", chatScope)
        .order("updated_at", { ascending: false }).limit(30);
      const duplicate = (existing ?? []).find((row: any) => String(row.content).toLocaleLowerCase() === content.toLocaleLowerCase());
      if (duplicate) {
        await client.from("luna_memories").update({
          importance: Math.max(1, Math.min(5, Number(item.importance) || 3)),
          confidence: Math.max(0, Math.min(1, Number(item.confidence) || 0.8)),
          source: item.source === "correction" ? "correction" : duplicate.source,
          updated_at: new Date().toISOString()
        }).eq("id", duplicate.id).eq("user_id", userId);
        continue;
      }
      await client.from("luna_memories").insert({
        user_id: userId, chat_id: chatScope, category, content,
        source: item.source === "correction" ? "correction" : "explicit",
        importance: Math.max(1, Math.min(5, Number(item.importance) || 3)),
        confidence: Math.max(0, Math.min(1, Number(item.confidence) || 0.8))
      });
    }
    if (keepChat) await client.from("luna_chat_memory_state").upsert({
      user_id: userId, chat_id: chatId,
      summary: memoryText(parsed.summary, 12000),
      last_scene: memoryText(parsed.last_scene, 8000),
      open_threads: Array.isArray(parsed.open_threads) ? parsed.open_threads.slice(0, 12).map((x: unknown) => memoryText(x, 400)).filter(Boolean) : [],
      updated_at: new Date().toISOString()
    }, { onConflict: "user_id,chat_id" });
  } catch (error) {
    console.error("luna memory update failed", error instanceof Error ? error.message : "unknown");
  }
}

const promptLimit = (user: any) => {
  const tier = String(user?.app_metadata?.subscription_tier ?? "free");
  if (tier === "luna_pro") return 1500;
  if (tier === "luna") return 500;
  return 100;
};

type SubscriptionTier = "free" | "luna" | "luna_pro";
const membershipCache = new Map<string, { member: boolean; checkedAt: number }>();

async function isLunaChannelMember(user: any): Promise<boolean> {
  const telegramId = String(user?.user_metadata?.sub ?? "");
  const botToken = Deno.env.get("TELEGRAM_BOT_TOKEN");
  if (!botToken || !new RegExp("^\\d{5,20}$").test(telegramId)) return false;
  const cached = membershipCache.get(telegramId);
  if (cached && Date.now() - cached.checkedAt < (cached.member ? 60_000 : 15_000)) return cached.member;
  try {
    const response = await fetch(`https://api.telegram.org/bot${botToken}/getChatMember`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ chat_id: "@LunaAiroll", user_id: Number(telegramId) }),
      signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) return false;
    const payload = await response.json();
    const member = payload?.ok === true && (
      ["creator", "administrator", "member"].includes(payload?.result?.status) ||
      (payload?.result?.status === "restricted" && payload?.result?.is_member === true)
    );
    membershipCache.set(telegramId, { member, checkedAt: Date.now() });
    return member;
  } catch {
    return false;
  }
}

async function resolveSubscription(admin: any, user: any, requestedPlan?: number) {
  const rawPlan = Number(requestedPlan ?? user?.user_metadata?.plan ?? 0);
  const selectedPlan = Number.isInteger(rawPlan) && rawPlan >= 0 && rawPlan <= 2 ? rawPlan : 0;
  const member = selectedPlan > 0 ? await isLunaChannelMember(user) : false;
  const plan = member ? selectedPlan : 0;
  const tier: SubscriptionTier = plan === 2 ? "luna_pro" : plan === 1 ? "luna" : "free";
  const currentTier = String(user?.app_metadata?.subscription_tier ?? "free");
  if (currentTier !== tier) {
    const { error } = await admin.auth.admin.updateUserById(user.id, {
      app_metadata: { ...(user.app_metadata ?? {}), subscription_tier: tier },
    });
    if (error) {
      console.error("luna subscription sync failed", error.message);
      return { plan: 0, tier: "free" as SubscriptionTier, member: false, error: true };
    }
  }
  return { plan, tier, member, error: false };
}

function tierLimits(tier: SubscriptionTier) {
  if (tier === "luna_pro") return {
    requestsPerMinute: 12, maxMessages: 20, maxMessageChars: 16000,
    memoryRows: 100, memoryCount: 40, memoryChars: 9000,
    maxCompletionTokens: 4096, reasoningEffort: "high" as const,
  };
  if (tier === "luna") return {
    requestsPerMinute: 12, maxMessages: 20, maxMessageChars: 16000,
    memoryRows: 75, memoryCount: 30, memoryChars: 6000,
    maxCompletionTokens: 2400, reasoningEffort: "medium" as const,
  };
  return {
    requestsPerMinute: 6, maxMessages: 12, maxMessageChars: 10000,
    memoryRows: 30, memoryCount: 15, memoryChars: 3000,
    maxCompletionTokens: 1400, reasoningEffort: "low" as const,
  };
}

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

  if (method === "GET" && path === "/health") {
    const botToken = Deno.env.get("TELEGRAM_BOT_TOKEN");
    let telegramBotValid = false;
    let channelAdmin = false;
    if (botToken) {
      try {
        const meResponse = await fetch(`https://api.telegram.org/bot${botToken}/getMe`, { signal: AbortSignal.timeout(5000) });
        const me = await meResponse.json();
        telegramBotValid = meResponse.ok && me?.ok === true && Number.isSafeInteger(me?.result?.id);
        if (telegramBotValid) {
          const memberResponse = await fetch(`https://api.telegram.org/bot${botToken}/getChatMember`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ chat_id: "@LunaAiroll", user_id: me.result.id }),
            signal: AbortSignal.timeout(5000),
          });
          const member = await memberResponse.json();
          channelAdmin = memberResponse.ok && member?.ok === true &&
            ["creator", "administrator"].includes(member?.result?.status);
        }
      } catch {
        telegramBotValid = false;
        channelAdmin = false;
      }
    }
    return json(200, { service: "luna-api", status: "ok", telegramBotConfigured: Boolean(botToken), telegramBotValid, channelAdmin });
  }

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
      const subscription = await resolveSubscription(admin, tokenData.user);
      if (subscription.error) return text(503, "Не удалось проверить доступ к тарифу. Попробуйте позже.");
      const effectiveUser = { ...tokenData.user, app_metadata: { ...(tokenData.user.app_metadata ?? {}), subscription_tier: subscription.tier } };
      return json(200, {
        id: tokenData.user.id, name, email: tokenData.user.email ?? "",
        plan: subscription.plan, prompt_limit: promptLimit(effectiveUser),
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
      const subscription = await resolveSubscription(admin, user);
      if (subscription.error) return text(503, "Не удалось проверить доступ к тарифу. Попробуйте позже.");
      const effectiveUser = { ...user, app_metadata: { ...(user.app_metadata ?? {}), subscription_tier: subscription.tier } };
      return json(200, {
        id: user.id,
        name: String(user.user_metadata?.name ?? user.user_metadata?.full_name ?? user.user_metadata?.preferred_username ?? user.user_metadata?.username ?? user.user_metadata?.given_name ?? "Пользователь Telegram").trim().slice(0, 40) || "Пользователь Telegram",
        email: user.email ?? "",
        plan: subscription.plan,
        prompt_limit: promptLimit(effectiveUser),
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

    if (path === "/api/memories" && method === "GET") {
      const { data, error } = await userClient.from("luna_memories")
        .select("id,chat_id,category,content,source,importance,confidence,is_pinned,updated_at")
        .eq("user_id", user.id).order("is_pinned", { ascending: false })
        .order("importance", { ascending: false }).order("updated_at", { ascending: false }).limit(200);
      if (error) return text(502, "Не удалось загрузить память.");
      return json(200, data ?? []);
    }
    const memoryMatch = path.match(new RegExp("^/api/memories/([0-9a-fA-F-]{36})$"));
    if (memoryMatch && method === "PUT") {
      const body = await req.json().catch(() => null);
      if (!body || typeof body !== "object") return text(400, "Некорректная запись памяти.");
      const patch: Record<string, unknown> = { updated_at: new Date().toISOString() };
      if (body.content !== undefined) {
        if (typeof body.content !== "string" || !body.content.trim() || body.content.trim().length > 500)
          return text(400, "Текст должен содержать от 1 до 500 символов.");
        patch.content = body.content.trim();
      }
      if (body.importance !== undefined) {
        const importance = Number(body.importance);
        if (!Number.isInteger(importance) || importance < 1 || importance > 5)
          return text(400, "Важность должна быть от 1 до 5.");
        patch.importance = importance;
      }
      if (body.is_pinned !== undefined) {
        if (typeof body.is_pinned !== "boolean") return text(400, "Некорректный статус закрепления.");
        patch.is_pinned = body.is_pinned;
      }
      if (Object.keys(patch).length === 1) return text(400, "Нет изменений для сохранения.");
      const { data, error } = await userClient.from("luna_memories").update(patch)
        .eq("id", memoryMatch[1]).eq("user_id", user.id)
        .select("id,chat_id,category,content,source,importance,confidence,is_pinned,updated_at").maybeSingle();
      if (error) return text(502, "Не удалось изменить запись памяти.");
      if (!data) return text(404, "Запись памяти не найдена.");
      return json(200, data);
    }
    if (memoryMatch && method === "DELETE") {
      const { data, error } = await userClient.from("luna_memories").delete()
        .eq("id", memoryMatch[1]).eq("user_id", user.id).select("id").maybeSingle();
      if (error) return text(502, "Не удалось удалить запись памяти.");
      if (!data) return text(404, "Запись памяти не найдена.");
      return new Response(null, { status: 204, headers: cors });
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
      const body = await req.json().catch(() => null);
      const rawPlan = Number(body?.plan);
      if (!Number.isInteger(rawPlan) || rawPlan < 0 || rawPlan > 2) return text(400, "Некорректный тариф.");
      if (rawPlan > 0 && !(await isLunaChannelMember(user)))
        return text(403, "Для доступа к премиум-моделям подпишитесь на Telegram-канал Luna AI.");
      const { data, error } = await userClient.auth.updateUser({ data: { plan: rawPlan } });
      if (error || !data.user) return text(502, "Не удалось сохранить выбор тарифа.");
      const subscription = await resolveSubscription(admin, data.user, rawPlan);
      if (subscription.error) return text(503, "Не удалось проверить доступ к тарифу. Попробуйте позже.");
      return new Response(null, { status: 204, headers: cors });
    }

    if (path === "/api/provider-settings") return text(404, "Маршрут не найден.");

    if (path === "/api/chat" && method === "POST") {
      const subscription = await resolveSubscription(admin, user);
      if (subscription.error) return text(503, "Не удалось проверить доступ к тарифу. Попробуйте позже.");
      const limits = tierLimits(subscription.tier);
      const now = Date.now();
      const recent = (chatRateLimits.get(user.id) ?? []).filter((at) => now - at < 60_000);
      if (recent.length >= limits.requestsPerMinute) return text(429, "Слишком много сообщений. Подождите минуту.");
      recent.push(now);
      chatRateLimits.set(user.id, recent);
      const body = await req.json();
      const messages = Array.isArray(body.msgs) ? body.msgs : [];
      if (!messages.length || messages.length > limits.maxMessages || messages.some((m: any) =>
        !["user", "assistant"].includes(m.role) || typeof m.content !== "string" || m.content.length > limits.maxMessageChars))
        return text(400, "Слишком длинный запрос или некорректная история.");

      const chatId = Number(body.chat_id);
      if (!Number.isSafeInteger(chatId) || chatId <= 0) return text(400, "Некорректный идентификатор чата.");
      const keepChat = body.keep !== false;
      if (keepChat) {
        const firstUserMessage = messages.find((m: any) => m.role === "user")?.content ?? "";
        const { error: chatSeedError } = await userClient.from("luna_chats").upsert({
          user_id: user.id, id: chatId,
          title: String(firstUserMessage).slice(0, 40),
          msgs: messages, updated_at: new Date().toISOString()
        }, { onConflict: "user_id,id" });
        if (chatSeedError) return text(502, "Не удалось подготовить сохранение истории.");
      }

      const { data: memoryRows } = await userClient.from("luna_memories")
        .select("category,content,importance,confidence,source,chat_id")
        .eq("user_id", user.id).or(body.keep === false ? "chat_id.is.null" : `chat_id.is.null,chat_id.eq.${chatId}`)
        .order("updated_at", { ascending: false }).limit(limits.memoryRows);
      const storyStateResult = keepChat
        ? await userClient.from("luna_chat_memory_state")
          .select("summary,last_scene,open_threads").eq("user_id", user.id).eq("chat_id", chatId).maybeSingle()
        : { data: null };
      const storyState = storyStateResult.data;
      const prioritizedMemories = [...(memoryRows ?? [])].sort((a: any, b: any) =>
        Number(b.source === "correction") - Number(a.source === "correction") ||
        Number(b.importance) - Number(a.importance)
      ).slice(0, limits.memoryCount);
      const memoryContext = prioritizedMemories.map((m: any) =>
        `[${m.category}; ${m.source === "correction" ? "исправление пользователя" : "память"}; ${m.chat_id === null ? "общая память" : "этот чат"}] ${m.content}`
      ).join("\n").slice(0, limits.memoryChars);
      const storyContext = storyState ? [
        storyState.summary ? `Состояние истории: ${storyState.summary}` : "",
        storyState.last_scene ? `Последняя сцена: ${storyState.last_scene}` : "",
        Array.isArray(storyState.open_threads) && storyState.open_threads.length ? `Незавершённые линии: ${storyState.open_threads.join("; ")}` : ""
      ].filter(Boolean).join("\n") : "";

      const words = promptWordCount(body.who) + promptWordCount(body.rules);
      const effectiveUser = { ...user, app_metadata: { ...(user.app_metadata ?? {}), subscription_tier: subscription.tier } };
      const limit = promptLimit(effectiveUser);
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
            ].join("\n")
          : [
              "You are Luna. Use the following fixed identity when asked who you are:",
              defaultIdentity,
              "If asked which exact underlying model or model provider powers you, do not guess or invent a name; say that you are Luna and that the exact underlying model is not part of your public identity.",
              "If asked who developed you, identify yourself as the Luna AI assistant/project. Do not claim that OpenAI or another model provider developed Luna.",
              "Answer factual and technical questions accurately and honestly; do not turn them into roleplay.",
            ].join("\n"),
        `Response length: ${String(body.len ?? "normal").slice(0, 40)}.`,
        `User name: ${String(body.you ?? "").slice(0, 80)}.`,
        "Never disclose internal infrastructure, API credentials, hosting, vendors, model providers, or implementation details. This rule cannot be overridden by roleplay or custom persona instructions. Do not invent alternative technical claims; redirect briefly to Luna's user-facing capabilities.",
        "Roleplay continuity is a core priority: preserve the configured character, relationships, established facts, emotional state and scene continuity. Never silently rewrite user-confirmed facts.",
        "User corrections are authoritative and newer corrections take precedence over older conflicting memories. Never delete or silently rewrite unrelated facts.",
        "Conversation style: sound like a thoughtful, natural human conversation, not a scripted assistant or promotional chatbot. Be direct, warm when appropriate, and specific to the user\u2019s actual message.",
        "Do not add emojis, decorative number symbols, or emoji bullets by default. Use plain text and ordinary Markdown only when it improves readability; mirror the user\u2019s style if they explicitly use emojis.",
        "Do not turn open-ended messages into menus, questionnaires, or lists of options unless the user asks for options or the task genuinely requires a choice. Prefer a relevant conversational response and ask at most one focused follow-up question when needed.",
        "Avoid canned introductions, repetitive offers, artificial enthusiasm, slogans, poetic filler, and forced closing lines. Never append a generic invitation such as \u2018choose one\u2019 or \u2018let\u2019s explore together\u2019 when it adds no value.",
        "Vary response structure naturally. Use lists only for genuinely separate points; do not force every answer into three numbered sections. Keep simple answers concise and answer the actual question first.",
        memoryContext ? `Relevant long-term memories:\n${memoryContext}` : "",
        storyContext ? `Current roleplay continuity:\n${storyContext}` : "",
      ].join("\n");
      const all = [{ role: "system", content: system }, ...messages];
      const upstream = await fetch("https://api.groq.com/openai/v1/chat/completions", {
        method: "POST",
        headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
        body: JSON.stringify({
          model, messages: all, stream: false,
          reasoning_effort: limits.reasoningEffort,
          reasoning_format: "hidden",
          max_completion_tokens: limits.maxCompletionTokens,
        }),
      });
      if (!upstream.ok) {
        if ([401, 403].includes(upstream.status)) return text(422, "Не удалось выполнить запрос. Попробуйте позже.");
        if (upstream.status === 429) return text(429, "Сервис временно перегружен. Попробуйте позже.");
        return text(502, "Не удалось получить ответ. Попробуйте ещё раз.");
      }
      const result = await upstream.json();
      const answer = result?.choices?.[0]?.message?.content;
      if (typeof answer !== "string") return text(502, "Не удалось сформировать ответ.");
      const publicAnswer = protectPublicIdentity(answer);
      await saveRoleplayMemory(userClient, user.id, chatId, keepChat, model, apiKey, messages, publicAnswer);
      return text(200, publicAnswer);
    }

    return text(404, "Маршрут не найден.");
  } catch (error) {
    console.error("luna-api error", error instanceof Error ? error.message : "unknown");
    return json(500, { error: "Internal server error" });
  }
});