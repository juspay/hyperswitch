import {randomUUID} from 'node:crypto';
import {defaultGrafanaPanels} from '../src/grafana-panels.mjs';

/** Accept private Grafana servers while rejecting credentials and cloud metadata endpoints. */
export function grafanaUrl(value){
 const url=new URL(String(value||'').trim());
 if(!['http:','https:'].includes(url.protocol)||url.username||url.password||url.search||url.hash)throw new Error('Enter a Grafana HTTP(S) server URL without credentials or query parameters');
 if(['169.254.169.254','100.100.100.200','[fd00:ec2::254]','metadata.google.internal'].includes(url.hostname))throw new Error('Cloud metadata endpoints cannot be used as Grafana servers');
 if(!['','/grafana'].includes(url.pathname.replace(/\/$/,'')))throw new Error('Use the Grafana server root or its /grafana subpath');
 return url.href.replace(/\/$/,'');
}
/** Resolve a draft identity without forwarding a saved token to a different server. */
export function grafanaConnection(draft={},saved={}){
 const baseUrl=grafanaUrl(draft.baseUrl??saved.baseUrl??process.env.GRAFANA_URL??'http://localhost:3002');
 const binding=saved.credentialOrigin||process.env.GRAFANA_URL||'http://localhost:3002';
 const supplied=String(draft.token||'').trim();
 if(supplied.length>4096||/[\r\n]/.test(supplied))throw new Error('Invalid Grafana service-account token');
 const token=supplied||(grafanaUrl(binding)===baseUrl?(saved.token??process.env.GRAFANA_TOKEN??''):'');
 const user=!supplied&&grafanaUrl(binding)===baseUrl?(saved.authProxyUser??process.env.GRAFANA_USER??''):'';
 if(!token&&!user)throw new Error('Enter a Viewer service-account token for this Grafana server');
 return {baseUrl,token,authProxyUser:user,credentialOrigin:baseUrl};
}
/** Replace retired k6 presets once, preserving custom panels and intentionally empty lists. */
export function migrateGrafanaPanels(settings={}){
 if(settings.presetsVersion===2)return settings.panels||[];
 const custom=(settings.panels||[]).filter(p=>!/^\/grafana\/d-solo\/k6\//.test(p.url));
 return custom.length?custom:defaultGrafanaPanels();
}
/** Keep panel identity and dashboard variables; pasted hosts and transient credentials are discarded. */
export function normalizeGrafanaPanel(value,baseUrl){
 const url=new URL(String(value||'').trim(),grafanaUrl(baseUrl));
 if(!['http:','https:'].includes(url.protocol)||url.username||url.password||url.href.length>8192)throw new Error('Paste a Grafana panel URL');
 const match=url.pathname.match(/^\/(?:grafana\/)?(?:d|d-solo)\/([A-Za-z0-9_-]{1,128})(?:\/([^/]*))?\/?$/);
 const selected=url.searchParams.get('panelId')||url.searchParams.get('viewPanel')||'';
 const panelId=selected.replace(/^panel-/,'');
 if(!match||!/^\d{1,10}$/.test(panelId)||Number(panelId)>2147483647)throw new Error('Select a panel in Grafana and paste its panel link');
 const query=new URLSearchParams({panelId:String(Number(panelId))});
 for(const [key,val]of url.searchParams)if(/^var-[\w.-]{1,80}$/.test(key))query.append(key,val);
 if([...query].length>100)throw new Error('Too many panel variables');
 return {dashboardUid:match[1],panelId:Number(panelId),url:`/grafana/d-solo/${match[1]}/${encodeURIComponent(decodeURIComponent(match[2]||'panel'))}?${query}`};
}
/** Validate the persisted private bundle and invalidate embeds only when it changes. */
export function prepareGrafanaSettings(draft,saved){
 let connection;
 try{connection=grafanaConnection(draft,saved);}catch(error){
  // An unconfigured existing installation can still save unrelated settings.
  if(draft.token||grafanaUrl(draft.baseUrl)!==grafanaUrl(saved.baseUrl)||saved.token||saved.authProxyUser)throw error;
  connection={baseUrl:grafanaUrl(draft.baseUrl),token:'',authProxyUser:'',credentialOrigin:grafanaUrl(draft.baseUrl)};
 }
 if(!Array.isArray(draft.panels)||draft.panels.length>20)throw new Error('Choose up to 20 Grafana panels');
 const seen=new Set(),panels=draft.panels.map(panel=>{
  const parsed=normalizeGrafanaPanel(panel.url,connection.baseUrl);const key=parsed.url;
  if(seen.has(key))throw new Error('This Grafana panel is already added');seen.add(key);
  return {...parsed,title:String(panel.title||`Panel ${parsed.panelId}`).slice(0,150),...(panel.dashboardTitle?{dashboardTitle:String(panel.dashboardTitle).slice(0,150)}:{})};
 });
 const next={...connection,panels,presetsVersion:2};
 const comparable=({revision=undefined,...v})=>JSON.stringify(v);
 return {...next,revision:comparable(next)===comparable(saved)?saved.revision:randomUUID()};
}
async function grafanaJson(connection,path,{fetcher=fetch}={}){
 const headers={accept:'application/json',...(connection.token?{authorization:`Bearer ${connection.token}`}:{'x-webauth-user':connection.authProxyUser})};
 // Detect root hosting using the same transport as embedded panels.
 const {upstreamPrefix}=await import('./grafana.mjs');const base=new URL(connection.baseUrl);
 const prefix=await upstreamPrefix(base,headers);
 let response;
 try{response=await fetcher(new URL(prefix+path,base.origin),{headers,redirect:'manual',signal:AbortSignal.timeout(10000)});}catch{throw new Error('Grafana is unreachable from the console host');}
 if([401,403].includes(response.status)){await response.body?.cancel();throw new Error('Grafana access was denied. Check the Viewer token and dashboard permissions');}
 if(response.status===404){await response.body?.cancel();throw new Error('Dashboard or Grafana API was not found');}
 if(!response.ok||!response.headers.get('content-type')?.includes('application/json')){await response.body?.cancel();throw new Error('Grafana did not return an authenticated API response');}
 return response.json();
}
/** Verify authenticated API access; Grafana service accounts legitimately have user ID zero. */
export async function testGrafanaConnection(connection,options){
 const user=await grafanaJson(connection,'/api/user',options);
 if(!Number.isInteger(user.id)||user.id<0||!user.orgId||user.isSignedIn===false)throw new Error('Grafana did not authenticate the configured identity');
 return {ok:true,message:'Connected · Grafana access verified'};
}
/** Resolve panel metadata on the configured server, including panels nested in collapsed rows. */
export async function resolveGrafanaPanel(connection,value,options){
 const parsed=normalizeGrafanaPanel(value,connection.baseUrl);
 const response=await grafanaJson(connection,'/api/dashboards/uid/'+encodeURIComponent(parsed.dashboardUid),options);
 const flatten=panels=>(panels||[]).flatMap(p=>[p,...flatten(p.panels)]);
 const panel=flatten(response.dashboard?.panels).find(p=>p.id===parsed.panelId&&p.type!=='row');
 if(!panel)throw new Error(`Panel ${parsed.panelId} was not found in this dashboard`);
 return {...parsed,title:String(panel.title||`Panel ${parsed.panelId}`).slice(0,150),dashboardTitle:String(response.dashboard.title||'').slice(0,150)};
}
