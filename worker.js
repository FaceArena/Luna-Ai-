const SUPABASE_URL = "https://efycijufegudtvvhnxzv.supabase.co";
const SUPABASE_PUBLISHABLE_KEY = "sb_publishable_FubYDvcEUqV5TyY7UGfbOA_n3o2BmoU";

export default {
  async fetch(request, env) {
    const incoming = new URL(request.url);

    if (incoming.pathname.startsWith("/api/")) {
      const edgePath = incoming.pathname === "/api/health" ? "/health" : incoming.pathname;
      const target = `${SUPABASE_URL}/functions/v1/luna-api${edgePath}${incoming.search}`;
      const headers = new Headers({ apikey: SUPABASE_PUBLISHABLE_KEY });

      for (const name of ["cookie", "authorization", "content-type"]) {
        const value = request.headers.get(name);
        if (value) headers.set(name, value);
      }

      const init = { method: request.method, headers };
      if (request.method !== "GET" && request.method !== "HEAD") {
        init.body = await request.arrayBuffer();
      }

      try {
        const upstream = await fetch(target, init);
        const responseHeaders = new Headers();
        for (const name of ["content-type", "cache-control"]) {
          const value = upstream.headers.get(name);
          if (value) responseHeaders.set(name, value);
        }
        // Set-Cookie is a multi-value header. Preserve each cookie explicitly
        // when proxying Supabase responses through the Worker.
        const cookies = typeof upstream.headers.getSetCookie === "function"
          ? upstream.headers.getSetCookie()
          : [upstream.headers.get("set-cookie")].filter(Boolean);
        for (const cookie of cookies) responseHeaders.append("Set-Cookie", cookie);
        if (incoming.pathname === "/api/auth/telegram/exchange") {
          responseHeaders.set("Cache-Control", "no-store");
        }
        const body = request.method === "HEAD" || upstream.status === 204 ? null : upstream.body;
        return new Response(body, { status: upstream.status, headers: responseHeaders });
      } catch {
        return new Response("Luna API upstream is unavailable.", {
          status: 502,
          headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" },
        });
      }
    }

    return env.ASSETS.fetch(request);
  },
};
