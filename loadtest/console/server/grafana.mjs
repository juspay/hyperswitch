const prefixes = new Map();

// Discover root-hosted Grafana without changing its deployment. A configured
// subpath is explicit; otherwise probe health once per minute, never per asset.
export async function upstreamPrefix(base, headers) {
  if (base.pathname.replace(/\/$/, '') === '/grafana') return '/grafana';
  const cached = prefixes.get(base.origin);
  if (cached && cached.until > Date.now()) return cached.promise;
  const promise = (async () => {
    try {
      const response = await fetch(new URL('/api/health', base), {
        headers, redirect: 'manual', signal: AbortSignal.timeout(5000)
      });
      if (response.ok && response.headers.get('content-type')?.includes('application/json')) {
        const health = await response.json();
        if (health.database || health.version) return '';
      } else await response.body?.cancel();
    } catch { /* Existing deployments use the /grafana subpath. */ }
    return '/grafana';
  })();
  prefixes.set(base.origin, {promise, until: Date.now() + 60000});
  return promise;
}

export function rewriteGrafana(text, origin) {
  // Grafana's bootstrap supplies paths for API calls and webpack chunks.
  // Only path settings are changed; the token remains in server request headers.
  return text
    .replace(/(<base\b[^>]*href=["'])\/(?:["'])/gi, '$1/grafana/"')
    .replace(/"appSubUrl"\s*:\s*""/g, '"appSubUrl":"/grafana"')
    .replace(/"appUrl"\s*:\s*"[^"\n]*"/g, `"appUrl":${JSON.stringify(origin + '/grafana/')}`)
    .replace(/((?:src|href|action)=["'])\/(?!\/|grafana(?:\/|["']))/gi, '$1/grafana/')
    .replace(/"liveEnabled"\s*:\s*true/g, '"liveEnabled":false');
}
