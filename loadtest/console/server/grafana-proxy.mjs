import {grafanaConnection} from './grafana-settings.mjs';
import {localSettings,httpUrl} from './state.mjs';
const json=(value,init)=>Response.json(value,init);
import { upstreamPrefix, rewriteGrafana } from './grafana.mjs';

export async function proxyGrafana(request,url,{connection=undefined,routePrefix='/grafana'}={}) {
  // Only the configured origin is reachable. Incoming identity headers and cookies are discarded.
  try{connection ||= grafanaConnection({},localSettings().grafana);}catch(error){return json({error:error.message},{status:process.env.GRAFANA_TOKEN||process.env.GRAFANA_USER||localSettings().grafana.token?403:503});}
  const base=httpUrl(connection.baseUrl);
  let destination=new URL(url.pathname+url.search,base.origin);
  if(destination.origin!==base.origin || !destination.pathname.startsWith('/grafana/'))return json({error:'Invalid Grafana path'},{status:400});
  if(request.method==='POST' && !/^\/grafana\/api\/(ds\/query$|datasources\/proxy\/)/.test(destination.pathname))return json({error:'Read-only Grafana proxy'},{status:403});
  const headers=new Headers();
  headers.set('accept',request.headers.get('accept') || '*/*');
  if(request.headers.get('content-type'))headers.set('content-type',request.headers.get('content-type'));
  if(connection.token)headers.set('authorization',`Bearer ${connection.token}`);
  if(connection.authProxyUser)headers.set('x-webauth-user',connection.authProxyUser);
  const prefix=await upstreamPrefix(base,headers);
  destination=new URL(prefix+url.pathname.slice('/grafana'.length)+url.search,base.origin);
  const body=request.method==='POST'?await request.arrayBuffer():undefined;
  if(body?.byteLength>1024*1024)return json({error:'Request too large'},{status:413});
  try {
    const upstream=await fetch(destination,{method:request.method,headers,body,redirect:'manual',signal:AbortSignal.any([request.signal,AbortSignal.timeout(30000)])});
    const outgoing=new Headers();
    for(const key of ['content-type','content-disposition','content-security-policy'])if(upstream.headers.has(key))outgoing.set(key,upstream.headers.get(key));
    const location=upstream.headers.get('location');
    if(location) {
      const redirect=new URL(location,destination);
      if(redirect.origin!==base.origin || (prefix && !redirect.pathname.startsWith('/grafana/')))return json({error:'Grafana redirect is outside configured server'},{status:502});
      outgoing.set('location',routePrefix+(prefix?redirect.pathname.slice('/grafana'.length):redirect.pathname)+redirect.search);
    }
    outgoing.set('cache-control','no-store');
    outgoing.set('referrer-policy','no-referrer');
    outgoing.set('x-content-type-options','nosniff');
    const type=upstream.headers.get('content-type') || '';
    if(request.method!=='HEAD' && (type.includes('text/html') || destination.pathname===prefix+'/api/frontend/settings')) {
      return new Response(rewriteGrafana(await upstream.text(),url.origin,routePrefix),{status:upstream.status,headers:outgoing});
    }
    return new Response(upstream.body,{status:upstream.status,headers:outgoing});
  }catch{return json({error:'Grafana is unavailable'},{status:502});}
}

