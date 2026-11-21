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

function communicationStyleKey(value: string) {
  const match = /^Communication style\s*[—:-]\s*([a-z_]+)\s*:/i.exec(value);
  return match?.[1]?.toLowerCase() ?? "";
}
function isValidCommunicationStylePreference(value: string) {
  const match = /^Communication style\s*[—:-]\s*([a-z_]+)\s*:\s*([a-zа-яё _-]{1,40})$/i.exec(value);
  if (!match) return false;
  const key = match[1].toLowerCase();
  const preference = match[2].trim().toLowerCase();
  const allowed: Record<string, string[]> = {
    verbosity: ["concise", "balanced", "detailed"],
    tone: ["casual", "neutral", "formal", "warm", "professional"],
    humor: ["none", "light", "frequent"],
    emojis: ["none", "occasional", "frequent"],
    formatting: ["prose", "bullets", "structured"],
    directness: ["direct", "gentle", "diplomatic"],
    feedback: ["critical_direct", "balanced", "encouraging"],
  };
  if (key === "language") return /^[a-zа-яё][a-zа-яё -]{1,30}$/i.test(preference);
  return allowed[key]?.includes(preference) ?? false;
}
function detectCommunicationStyle(messages: any[]) {
  const userMessages = messages.filter((message: any) => message.role === "user" && typeof message.content === "string")
    .slice(-10).map((message: any) => message.content.slice(0, 3000));
  const found = new Map<string, { value: string; source: "explicit" | "inferred" }>();
  const rules: Array<{ key: string; value: string; pattern: RegExp }> = [
    { key: "verbosity", value: "concise", pattern: /(?:\b(?:keep|make|answer|reply|write)\b.{0,35}\b(?:brief|short|concise)\b|\bI prefer\b.{0,35}\b(?:brief|short|concise)\b|(?:отвечай|пиши|говори).{0,35}(?:кратко|коротко|без воды)|(?:предпочитаю|люблю).{0,35}(?:краткие|короткие).{0,25}(?:ответы|объяснения))/i },
    { key: "verbosity", value: "detailed", pattern: /(?:\b(?:keep|make|answer|reply|write)\b.{0,35}\b(?:detailed|in detail|thorough)\b|\bI prefer\b.{0,35}\b(?:detailed|thorough)\b|(?:отвечай|пиши|объясняй).{0,35}(?:подробно|развернуто|детально))/i },
    { key: "emojis", value: "none", pattern: /(?:\b(?:no emojis|without emojis|don't use emojis|do not use emojis)\b|без эмодзи|не используй эмодзи|не ставь смайлики)/i },
    { key: "emojis", value: "occasional", pattern: /(?:use emojis occasionally|a few emojis|иногда используй эмодзи|можно иногда эмодзи)/i },
    { key: "emojis", value: "frequent", pattern: /(?:\b(?:use emojis|add emojis)\b|используй эмодзи|добавляй смайлики)/i },
    { key: "tone", value: "casual", pattern: /(?:\b(?:be casual|talk casually|informal tone)\b|общайся неформально|на ты|пиши проще)/i },
    { key: "tone", value: "formal", pattern: /(?:\b(?:be formal|formal tone|address me formally)\b|общайся формально|обращайся на вы)/i },
    { key: "directness", value: "direct", pattern: /(?:\b(?:be direct|tell me straight|no sugarcoating)\b|говори прямо|без смягчений)/i },
    { key: "directness", value: "gentle", pattern: /(?:\b(?:be gentle|soften the tone)\b|помягче|мягче формулируй)/i },
    { key: "feedback", value: "critical_direct", pattern: /(?:\b(?:challenge my ideas|be a critical reviewer|don't just agree|do not just agree)\b|не поддакивай|ищи ошибки|критикуй прямо|строгий оппонент)/i },
    { key: "feedback", value: "encouraging", pattern: /(?:\b(?:be encouraging|encourage me)\b|поддерживай меня|больше поддержки)/i },
    { key: "humor", value: "none", pattern: /(?:\b(?:no jokes|don't joke|do not joke)\b|без шуток|не шути)/i },
    { key: "humor", value: "light", pattern: /(?:\blight humor\b|немного юмора|легкий юмор)/i },
    { key: "humor", value: "frequent", pattern: /(?:\b(?:be funny|joke more|use humor)\b|шути чаще|больше юмора)/i },
    { key: "formatting", value: "prose", pattern: /(?:\b(?:no bullet points|avoid lists|plain paragraphs)\b|без списков|обычным текстом)/i },
    { key: "formatting", value: "bullets", pattern: /(?:\b(?:use bullet points|use lists)\b|используй списки|пунктами)/i },
    { key: "language", value: "Russian", pattern: /(?:\b(?:reply|answer|write) in Russian\b|по-русски|на русском языке|отвечай на русском)/i },
    { key: "language", value: "English", pattern: /(?:\b(?:reply|answer|write) in English\b|по-английски|на английском языке|отвечай на английском)/i },
  ];
  const repeated = new Map<string, { key: string; value: string; count: number }>();
  const persistentIntent = /(?:\b(?:i prefer|i like|my preference|from now on|always|usually|generally|i want you to always)\b|предпочитаю|мне нравится|всегда|обычно|в дальнейшем|постоянно)/i;
  for (const message of userMessages) {
    for (const rule of rules) {
      if (!rule.pattern.test(message)) continue;
      if (persistentIntent.test(message)) {
        found.set(rule.key, { value: rule.value, source: "explicit" });
      } else {
        const signature = rule.key + ":" + rule.value;
        const previous = repeated.get(signature);
        repeated.set(signature, { key: rule.key, value: rule.value, count: (previous?.count ?? 0) + 1 });
      }
    }
  }
  for (const preference of repeated.values()) {
    if (preference.count >= 3 && !found.has(preference.key)) {
      found.set(preference.key, { value: preference.value, source: "inferred" });
    }
  }
  if (!found.has("language") && userMessages.length >= 3) {
    const votes = userMessages.slice(-5).map((message) => {
      const cyrillic = (message.match(/[А-Яа-яЁё]/g) ?? []).length;
      const latin = (message.match(/[A-Za-z]/g) ?? []).length;
      if (cyrillic + latin < 15) return "";
      if (cyrillic > latin * 1.2) return "Russian";
      if (latin > cyrillic * 1.2) return "English";
      return "";
    });
    const russianVotes = votes.filter((value) => value === "Russian").length;
    const englishVotes = votes.filter((value) => value === "English").length;
    if (russianVotes >= 3) found.set("language", { value: "Russian", source: "inferred" });
    else if (englishVotes >= 3) found.set("language", { value: "English", source: "inferred" });
  }
  return [...found.entries()].map(([key, preference]) => ({
    key, ...preference, content: "Communication style — " + key + ": " + preference.value,
  })).filter((preference) => isValidCommunicationStylePreference(preference.content));
}
async function saveCommunicationStyle(client: any, userId: string, preferences: any[]) {
  if (!preferences.length) return;
  const { data: existing, error } = await client.from("luna_memories")
    .select("id,content").eq("user_id", userId).eq("category", "preference")
    .is("chat_id", null).like("content", "Communication style%").limit(50);
  if (error) {
    console.error("luna communication profile read failed", error.message);
    return;
  }
  await Promise.all(preferences.map(async (preference) => {
    const current = (existing ?? []).find((row: any) =>
      isValidCommunicationStylePreference(String(row.content)) &&
      communicationStyleKey(String(row.content)) === preference.key
    );
    const values = {
      content: preference.content,
      source: preference.source,
      importance: preference.source === "explicit" ? 4 : 3,
      confidence: preference.source === "explicit" ? 1 : 0.75,
      updated_at: new Date().toISOString(),
    };
    if (current) {
      if (current.content === preference.content) return;
      const { error: updateError } = await client.from("luna_memories").update(values)
        .eq("id", current.id).eq("user_id", userId);
      if (updateError) console.error("luna communication profile update failed", updateError.message);
    } else {
      const { error: insertError } = await client.from("luna_memories").insert({
        user_id: userId, chat_id: null, category: "preference", ...values,
      });
      if (insertError) console.error("luna communication profile insert failed", insertError.message);
    }
  }));
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
  const admin = createClient(url, service, {
    auth: { persistSession: false, autoRefreshToken: false },
  });

  try {
    if (req.method === "GET") return reply(200, { status: "ready" });
    if (req.method !== "POST") return reply(405, { error: "Method not allowed" });
    const body = await req.json();
    if (body.action !== "chat") return reply(400, { error: "Unknown action" });

    const messages = Array.isArray(body.msgs) ? body.msgs : [];
    if (!messages.length || messages.length > 20 ||
        messages.some((m: any) => !["user", "assistant"].includes(m.role) ||
          typeof m.content !== "string" || m.content.length > 16000)) {
      return reply(400, { error: "Invalid chat messages" });
    }
    const { data: apiKey, error: keyError } = await admin.rpc("luna_get_global_secret", {
      p_name: "luna_global_groq_api_key",
    });
    if (keyError || typeof apiKey !== "string" || !apiKey) {
      return reply(503, { error: "AI service temporarily unavailable" });
    }
    const { data: styleRows, error: styleError } = await userClient.from("luna_memories")
      .select("content").eq("user_id", authData.user.id).eq("category", "preference")
      .is("chat_id", null).like("content", "Communication style%")
      .order("updated_at", { ascending: false }).limit(8);
    const styleContext = styleError ? "" : (styleRows ?? [])
      .map((row: any) => String(row.content))
      .filter(isValidCommunicationStylePreference).join("\n").slice(0, 1200);
    const baseSystem = typeof body.system === "string" ? body.system.slice(0, 8000) : "";
    const system = [
      baseSystem,
      styleContext ? "Stable user communication preferences (apply naturally, without announcing them):\n" + styleContext : "",
      styleContext ? "Always mirror the language of the user's current message unless they explicitly request another language. Use other learned preferences only when they do not conflict with the current request or custom instructions. Do not infer other personal traits." : "",
    ].filter(Boolean).join("\n").slice(0, 9500);
    const content: Array<{ role: string; content: string }> = [];
    if (system) content.push({ role: "system", content: system });
    content.push(...messages);
    const upstream = await fetch("https://api.groq.com/openai/v1/chat/completions", {
      method: "POST",
      headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
      body: JSON.stringify({ model: "openai/gpt-oss-120b", messages: content, stream: false }),
    });
    if (!upstream.ok) {
      if ([401, 403].includes(upstream.status)) return reply(422, { error: "AI service configuration rejected" });
      if (upstream.status === 429) return reply(429, { error: "AI service rate limit reached" });
      return reply(502, { error: "AI request failed" });
    }
    const result = await upstream.json();
    const answer = result?.choices?.[0]?.message?.content;
    if (typeof answer !== "string") return reply(502, { error: "Invalid AI response" });
    const learnedPreferences = detectCommunicationStyle(messages);
    await saveCommunicationStyle(userClient, authData.user.id, learnedPreferences);
    return new Response(answer, { status: 200, headers: { ...headers, "Content-Type": "text/plain; charset=utf-8" } });
  } catch (error) {
    console.error("luna-ai function error", error instanceof Error ? error.message : "unknown");
    return reply(500, { error: "Internal service error" });
  }
});
