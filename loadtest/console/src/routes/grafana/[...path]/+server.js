import { localSettings,httpUrl } from '../../../../server/state.mjs';
import { json } from '@sveltejs/kit';
import { upstreamPrefix, rewriteGrafana } from '../../../../server/grafana.mjs';

async function proxy({request,url}) {
  // Only the configured origin is reachable. Incoming identity headers and cookies are discarded.
  const base=httpUrl(localSettings().grafana.baseUrl);
  if(!process.env.GRAFANA_TOKEN && !process.env.GRAFANA_USER)return json({error:'Set GRAFANA_TOKEN in loadtest/console/.env to a Grafana Viewer service-account token, then restart the console. For configured Grafana auth proxy deployments, set GRAFANA_USER instead.'},{status:503});
  let destination=new URL(url.pathname+url.search,base.origin);
  if(destination.origin!==base.origin || !destination.pathname.startsWith('/grafana/'))return json({error:'Invalid Grafana path'},{status:400});
  if(request.method==='POST' && !/^\/grafana\/api\/(ds\/query$|datasources\/proxy\/)/.test(destination.pathname))return json({error:'Read-only Grafana proxy'},{status:403});
  const headers=new Headers();
  headers.set('accept',request.headers.get('accept') || '*/*');
  if(request.headers.get('content-type'))headers.set('content-type',request.headers.get('content-type'));
  if(process.env.GRAFANA_TOKEN)headers.set('authorization',`Bearer ${process.env.GRAFANA_TOKEN}`);
  if(process.env.GRAFANA_USER)headers.set('x-webauth-user',process.env.GRAFANA_USER);
  const prefix=await upstreamPrefix(base,headers);
  destination=new URL(prefix+url.pathname.slice('/grafana'.length)+url.search,base.origin);
  const body=request.method==='POST'?await request.arrayBuffer():undefined;
  if(body?.byteLength>1024*1024)return json({error:'Request too large'},{status:413});
  try {
    const upstream=await fetch(destination,{method:request.method,headers,body,redirect:'manual',signal:AbortSignal.any([request.signal,AbortSignal.timeout(30000)])});
    const outgoing=new Headers();
    for(const key of ['content-type','content-disposition'])if(upstream.headers.has(key))outgoing.set(key,upstream.headers.get(key));
    const location=upstream.headers.get('location');
    if(location) {
      const redirect=new URL(location,destination);
      if(redirect.origin!==base.origin || (prefix && !redirect.pathname.startsWith('/grafana/')))return json({error:'Grafana redirect is outside configured server'},{status:502});
      outgoing.set('location',(prefix?'':'/grafana')+redirect.pathname+redirect.search);
    }
    outgoing.set('cache-control','no-store');
    const type=upstream.headers.get('content-type') || '';
    if(!prefix && request.method!=='HEAD' && (type.includes('text/html') || destination.pathname==='/api/frontend/settings')) {
      return new Response(rewriteGrafana(await upstream.text(),url.origin),{status:upstream.status,headers:outgoing});
    }
    return new Response(upstream.body,{status:upstream.status,headers:outgoing});
  }catch{return json({error:'Grafana is unavailable'},{status:502});}
}
export const GET=proxy,HEAD=proxy,POST=proxy;
