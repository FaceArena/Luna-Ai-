export async function onRequest({ request }) {
  const supabaseUrl = "https://efycijufegudtvvhnxzv.supabase.co";
  // Supabase publishable keys are public by design; provider and service-role secrets stay server-side.
  const publishableKey = "sb_publishable_FubYDvcEUqV5TyY7UGfbOA_n3o2BmoU";

  const incoming = new URL(request.url);
  const edgePath = incoming.pathname === "/api/health" ? "/health" : incoming.pathname;
  const target = `${supabaseUrl}/functions/v1/luna-api${edgePath}${incoming.search}`;

  const headers = new Headers({ apikey: publishableKey });
  for (const name of ["cookie", "authorization", "content-type"]) {
    const value = request.headers.get(name);
    if (value) headers.set(name, value);
  }

  const method = request.method;
  const init = { method, headers };
  if (method !== "GET" && method !== "HEAD") {
    init.body = await request.arrayBuffer();
  }

  try {
    const upstream = await fetch(target, init);
    const responseHeaders = new Headers();
    for (const name of ["content-type", "cache-control", "set-cookie"]) {
      const value = upstream.headers.get(name);
      if (value) responseHeaders.set(name, value);
    }
    const body = method === "HEAD" || upstream.status === 204 ? null : upstream.body;
    return new Response(body, { status: upstream.status, headers: responseHeaders });
  } catch {
    return new Response("Luna API upstream is unavailable.", {
      status: 502,
      headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" },
    });
  }
}
