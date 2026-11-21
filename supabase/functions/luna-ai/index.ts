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
  const token = (req.headers.get("Authorization") ?? "").replace(/^Bearer\\s+/i, "");
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
    const system = typeof body.system === "string" ? body.system.slice(0, 8000) : "";
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
    return new Response(answer, { status: 200, headers: { ...headers, "Content-Type": "text/plain; charset=utf-8" } });
  } catch (error) {
    console.error("luna-ai function error", error instanceof Error ? error.message : "unknown");
    return reply(500, { error: "Internal service error" });
  }
});
