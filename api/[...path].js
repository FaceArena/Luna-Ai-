module.exports = async function handler(req, res) {
  // Supabase publishable keys are designed to be public; no service-role or provider secret is stored here.
  const supabaseUrl = "https://efycijufegudtvvhnxzv.supabase.co";
  const publishableKey = "sb_publishable_FubYDvcEUqV5TyY7UGfbOA_n3o2BmoU";

  const incoming = new URL(req.url || "/", `https://${req.headers.host || "localhost"}`);
  const edgePath = incoming.pathname === "/api/health"
    ? "/health"
    : incoming.pathname;
  const target = `${supabaseUrl.replace(/\/$/, "")}/functions/v1/luna-api${edgePath}${incoming.search}`;

  const headers = new Headers();
  headers.set("apikey", publishableKey);
  if (req.headers.cookie) headers.set("cookie", req.headers.cookie);
  if (req.headers.authorization) headers.set("authorization", req.headers.authorization);
  if (req.headers["content-type"]) headers.set("content-type", req.headers["content-type"]);

  const method = req.method || "GET";
  let body;
  if (method !== "GET" && method !== "HEAD") {
    if (typeof req.body === "string") body = req.body;
    else if (req.body !== undefined) body = JSON.stringify(req.body);
    else {
      const chunks = [];
      for await (const chunk of req) chunks.push(Buffer.from(chunk));
      body = Buffer.concat(chunks);
    }
  }

  try {
    const upstream = await fetch(target, { method, headers, body });
    res.statusCode = upstream.status;
    const contentType = upstream.headers.get("content-type");
    if (contentType) res.setHeader("content-type", contentType);
    const setCookie = upstream.headers.get("set-cookie");
    if (setCookie) res.setHeader("set-cookie", setCookie);
    const cacheControl = upstream.headers.get("cache-control");
    if (cacheControl) res.setHeader("cache-control", cacheControl);
    if (method === "HEAD" || upstream.status === 204) {
      res.end();
      return;
    }
    res.end(Buffer.from(await upstream.arrayBuffer()));
  } catch (_error) {
    res.statusCode = 502;
    res.end("Luna API upstream is unavailable.");
  }
};
