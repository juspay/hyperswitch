import {httpUrl} from './state.mjs';

export async function testServiceEndpoint(kind,endpoint,{fetcher=fetch}={}){
  if(!['router','modularPm','superposition'].includes(kind))throw new Error('Unknown service');
  // Service endpoints may include an API prefix (for example `/v2` locally
  // or `/v1` in cloud). Preserve that prefix when constructing the probe URL.
  const base=httpUrl(endpoint),url=new URL(base);
  url.pathname=`${url.pathname.replace(/\/$/,'')}/health`;
  const started=performance.now();
  let response;
  try{response=await fetcher(url,{method:'GET',redirect:'manual',signal:AbortSignal.timeout(5000)});}
  catch(e){throw new Error(`${kind==='modularPm'?'Payment methods':kind==='superposition'?'Superposition':'Router'} is unreachable: ${e.message}`);}
  await response.body?.cancel().catch(()=>{});
  const latencyMs=Math.round(performance.now()-started);
  if(!response.ok)throw new Error(`Health check returned HTTP ${response.status}`);
  return {ok:true,status:response.status,latencyMs,message:`Reachable · HTTP ${response.status} · ${latencyMs} ms`};
}

export async function testSuperpositionEndpoint(config,{fetcher=fetch}={}){
  const key=Object.keys(config.defaults || {})[0];
  if(!key)throw new Error('Configure at least one Superposition default before testing');
  if(!config.token)throw new Error('Configure a Superposition token before testing');
  const base=httpUrl(config.endpoint),url=new URL(`/default-config/${encodeURIComponent(key)}`,base);
  const auth=config.access_header==='x-superposition-access'?{'x-superposition-access':config.token}:{Authorization:`Bearer ${config.token}`};
  const started=performance.now();
  let response;
  try{response=await fetcher(url,{method:'GET',headers:{...auth,'x-org-id':config.organization_id,'x-workspace':config.workspace_id},redirect:'manual',signal:AbortSignal.timeout(5000)});}
  catch(e){throw new Error(`Superposition is unreachable: ${e.message}`);}
  await response.body?.cancel().catch(()=>{});
  const latencyMs=Math.round(performance.now()-started);
  if(!response.ok)throw new Error(`Authenticated config check returned HTTP ${response.status}`);
  return {ok:true,status:response.status,latencyMs,message:`Authenticated · HTTP ${response.status} · ${latencyMs} ms`};
}
