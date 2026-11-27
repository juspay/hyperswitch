import {createHash,randomBytes} from 'node:crypto';
import {createServer} from 'node:http';
import {Readable} from 'node:stream';
import {pipeline} from 'node:stream/promises';
import {controlDb,localSettings,panelPath} from './state.mjs';
import {grafanaEmbedOrigin,grafanaProxyPort} from './grafana-origin.mjs';
import {proxyGrafana} from './grafana-proxy.mjs';
import {boundedRequest} from './request-body.mjs';
const digest=value=>createHash('sha256').update(String(value)).digest('hex');
const cookieName='loadtest_grafana';
let initialized=false;
function db(){const db=controlDb();if(!initialized){db.exec(`CREATE TABLE IF NOT EXISTS grafana_access(token_hash TEXT PRIMARY KEY,session_hash TEXT NOT NULL,expires INTEGER NOT NULL,pending INTEGER NOT NULL)`);initialized=true;}return db;}
export function createGrafanaEmbed(sessionToken,panel){
 const database=db(),now=Date.now(),sessionHash=digest(sessionToken||''),session=database.prepare('SELECT expires FROM sessions WHERE token=? AND expires>?').get(sessionHash,now);
 if(!session)throw Object.assign(new Error('Authentication required'),{status:401});
 const settings=localSettings(),route=panelPath(panel,settings.grafana.baseUrl),ticket=randomBytes(32).toString('base64url');
 database.prepare('DELETE FROM grafana_access WHERE expires<=? OR session_hash NOT IN (SELECT token FROM sessions)').run(now);
 database.prepare('INSERT INTO grafana_access VALUES(?,?,?,1)').run(digest(ticket),sessionHash,Math.min(session.expires,now+60000));
 const url=new URL(route,grafanaEmbedOrigin());url.searchParams.set('embed_ticket',ticket);
 return {url:url.href};
}
function authenticate(request,url){
 const database=db(),now=Date.now(),ticket=url.searchParams.get('embed_ticket');
 if(ticket){
  const access=database.prepare('SELECT a.token_hash,s.expires FROM grafana_access a JOIN sessions s ON s.token=a.session_hash WHERE a.token_hash=? AND a.pending=1 AND a.expires>? AND s.expires>?').get(digest(ticket),now,now);
  if(!access)return null;
  const token=randomBytes(32).toString('base64url');
  if(!database.prepare('UPDATE grafana_access SET token_hash=?,pending=0,expires=? WHERE token_hash=? AND pending=1').run(digest(token),access.expires,access.token_hash).changes)return null;
  url.searchParams.delete('embed_ticket');
  return new Response(null,{status:303,headers:{location:url.pathname+url.search,'set-cookie':`${cookieName}=${token}; Path=/grafana/; HttpOnly; SameSite=Strict; Max-Age=${Math.max(0,Math.floor((access.expires-now)/1000))}${url.protocol==='https:'?'; Secure':''}`,'referrer-policy':'no-referrer','cache-control':'no-store'}});
 }
 const token=request.headers.get('cookie')?.split(';').map(s=>s.trim()).find(s=>s.startsWith(cookieName+'='))?.slice(cookieName.length+1);
 return token&&database.prepare('SELECT 1 FROM grafana_access a JOIN sessions s ON s.token=a.session_hash WHERE a.token_hash=? AND a.pending=0 AND a.expires>? AND s.expires>?').get(digest(token),now,now)?true:null;
}
export async function grafanaEmbedRequest(request){
 const url=new URL(request.url),origin=grafanaEmbedOrigin();
 if(url.origin!==origin||!url.pathname.startsWith('/grafana/')||!['GET','HEAD','POST'].includes(request.method))return Response.json({error:'Not found'},{status:404});
 const auth=authenticate(request,url);if(auth instanceof Response)return auth;
 if(!auth)return Response.json({error:'Grafana embed session required'},{status:401});
 if(request.method==='POST'&&request.headers.get('origin')!==origin)return Response.json({error:'Invalid request origin'},{status:403});
 try{return await proxyGrafana(await boundedRequest(request),url);}catch(error){return Response.json({error:error.status===413?error.message:'Grafana is unavailable'},{status:error.status===413?413:502,headers:{connection:'close'}});}
}
export async function startGrafanaProxy(){
 const origin=grafanaEmbedOrigin();
 const server=createServer(async(req,res)=>{
  const abort=new AbortController();req.once('aborted',()=>abort.abort());res.once('close',()=>{if(!res.writableEnded)abort.abort();});
  try{
   const headers=new Headers();for(const [key,value] of Object.entries(req.headers))if(value)headers.set(key,Array.isArray(value)?value.join(','):value);
   const url=new URL(req.url,origin);if(url.origin!==origin)throw Error('Invalid origin');
   const options={method:req.method,headers,signal:abort.signal,...(!['GET','HEAD'].includes(req.method)?{body:/** @type {ReadableStream} */ (/** @type {unknown} */ (Readable.toWeb(req))),duplex:'half'}:{})};
   const request=new Request(url,options);
   const response=await grafanaEmbedRequest(request);res.writeHead(response.status,Object.fromEntries(response.headers));
   if(response.body&&req.method!=='HEAD')await pipeline(Readable.fromWeb(/** @type {import('node:stream/web').ReadableStream} */ (/** @type {unknown} */ (response.body))),res);else res.end();
  }catch{if(!res.headersSent)res.writeHead(502);res.end();}
 });
 await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(grafanaProxyPort(),process.env.GRAFANA_PROXY_HOST||process.env.HOST||'127.0.0.1',()=>resolve(undefined));});
 return server;
}
