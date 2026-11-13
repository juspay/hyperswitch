import {createHash,randomUUID,timingSafeEqual} from 'node:crypto';
import path from 'node:path';
import {mkdir,readFile} from 'node:fs/promises';
import {controlDb,dataRoot,configPath,readJsonSync,atomicJson,activeEnvironment} from './state.mjs';
import {connectorSet,normalizeConnectorPlan} from '../src/connector-config.mjs';
import {requireSupportedEnvironment} from '../src/environments.mjs';

// The same requirement projection is used at verification and run admission.
export function fixtureRequirements(config){
 const enabled=(config.scenarios||[]).filter(f=>Number(f.weight)>0);
 return {recurring:enabled.some(f=>['cit_off_session','cit_metadata_changed','mit','saved_card_checkout'].includes(f.scenario)),publishableKey:enabled.some(f=>f.merchant_path!=='legacy'||f.scenario==='sdk_checkout'),connectors:connectorSet(normalizeConnectorPlan(config.connectors))};
}
export function normalizeImportedMerchants(input,count){
 if(!Array.isArray(input)||input.length!==count)throw new Error(`Import exactly ${count} merchant${count===1?'':'s'}`);
 const ids=new Set();return input.map((row,i)=>{
  if(!row||typeof row!=='object')throw new Error(`Merchant ${i+1}: expected an object`);
  const result={};
  for(const key of ['merchant_id','profile_id','api_key','publishable_key']){
   const value=row[key]??'';if(typeof value!=='string'||value.length>16384||/[\r\n\0]/.test(value))throw new Error(`Merchant ${i+1}: invalid ${key}`);
   if(key!=='publishable_key'&&!value.trim())throw new Error(`Merchant ${i+1}: ${key} is required`);
   if(key.endsWith('_id')&&!/^[a-zA-Z0-9_-]+$/.test(value))throw new Error(`Merchant ${i+1}: invalid ${key}`);
   result[key]=value;
  }
  if(ids.has(result.merchant_id))throw new Error(`Merchant ${i+1}: duplicate merchant ID`);ids.add(result.merchant_id);
  const accounts=row.connector_accounts??{};
  if(typeof accounts!=='object'||Array.isArray(accounts)||Object.entries(accounts).some(([k,v])=>!/^\w+$/.test(k)||typeof v!=='string'||!/^\w+$/.test(v)))throw new Error(`Merchant ${i+1}: invalid connector account mapping`);
  result.connector_accounts={...accounts};result.merchant_connector_id=Object.values(accounts)[0]||'';return result;
 });
}
export function importFingerprint(config,merchants){
 return createHash('sha256').update(JSON.stringify({environment:config.environment,router:config.services.router,pm:config.services.modular_pm,admin:config.fixtures.admin_api_key,superposition:{endpoint:config.fixtures.superposition?.endpoint,organization_id:config.fixtures.superposition?.organization_id,workspace_id:config.fixtures.superposition?.workspace_id,defaults:config.fixtures.superposition?.defaults,context:config.fixtures.superposition?.context},requirements:fixtureRequirements(config),merchants})).digest('hex');
}
const equal=(a,b)=>{if(typeof a!=='string'||typeof b!=='string')return false;const x=Buffer.from(a),y=Buffer.from(b);return x.length===y.length&&timingSafeEqual(x,y);};
export async function verifyImportedMerchants(config,merchants,{fetcher=fetch,signal=null,onProgress=(_checks,_state)=>{}}={}){
 const requirements=fixtureRequirements(config),checks=[],accounts=new Map();
 let phase='credentials';
 const stages={credentials:{status:'running',completed:0,total:merchants.length*(requirements.publishableKey?3:2),failed:0},setup:{status:'pending',completed:0,total:merchants.length*requirements.connectors.length,failed:0}};
 const report=()=>onProgress(checks,{phase,stages:structuredClone(stages),checked:checks.length});
 const check=async(merchant,stage,label,fn)=>{
  signal?.throwIfAborted();let result;
  try{await fn();signal?.throwIfAborted();result={merchant,stage,label,ok:true,message:'Verified'};}
  catch(e){if(signal?.aborted)throw e;result={merchant,stage,label,ok:false,message:e.message};}
  checks.push(result);stages[stage].completed++;if(!result.ok)stages[stage].failed++;
  await report();
 };
 const get=async(route,headers)=>{
  const url=`${config.services.router.replace(/\/$/,'')}${route}`;let response;
  try{response=await fetcher(url,{headers,redirect:'manual',signal:signal?AbortSignal.any([signal,AbortSignal.timeout(8000)]):AbortSignal.timeout(8000)});}catch{throw new Error('Hyperswitch is unreachable or the request timed out');}
  if(!response.ok){await response.body?.cancel();throw new Error(`Hyperswitch returned HTTP ${response.status}${response.status===401||response.status===403?' — credentials or inspection permissions were rejected':''}`);}
  try{return await response.json();}catch{throw new Error('Hyperswitch returned an invalid inspection response');}
 };
 const parallel=async inspect=>{
  let cursor=0;const workers=await Promise.allSettled(Array.from({length:Math.min(4,merchants.length)},async()=>{while(cursor<merchants.length){signal?.throwIfAborted();await inspect(merchants[cursor++]);}}));
  const failed=workers.find(result=>result.status==='rejected');if(failed?.status==='rejected')throw failed.reason;
 };
 await report();
 await parallel(async row=>{
  const mid=encodeURIComponent(row.merchant_id),pid=encodeURIComponent(row.profile_id);
  await check(row.merchant_id,'credentials','Merchant API key',async()=>{const connectors=await get(`/account/${mid}/connectors`,{'api-key':row.api_key});if(!Array.isArray(connectors))throw new Error('Connector inspection did not return an account list');accounts.set(row.merchant_id,connectors);});
  await check(row.merchant_id,'credentials','Merchant and profile ownership',async()=>{
   const profile=await get(`/account/${mid}/business_profile/${pid}`,{'api-key':row.api_key});
   if(profile.profile_id!==row.profile_id||profile.merchant_id!==row.merchant_id)throw new Error('Profile does not belong to this merchant');
  });
  if(requirements.publishableKey)await check(row.merchant_id,'credentials','Publishable key',async()=>{
   if(!row.publishable_key)throw new Error('Publishable key is required by the selected payment flows');
   if(!config.fixtures.admin_api_key)throw new Error('Configure the Router admin key to verify the publishable key');
   const admin={'api-key':config.fixtures.admin_api_key,Authorization:`admin-api-key=${config.fixtures.admin_api_key}`};
   const account=await get(`/accounts/${mid}`,admin);
   if(account.merchant_id!==row.merchant_id||!equal(account.publishable_key,row.publishable_key))throw new Error('Publishable key does not match this merchant');
  });
 });
 if(stages.credentials.failed){
  stages.credentials.status='failed';stages.setup.status='blocked';phase='failed';await report();
  return {ok:false,checks,phase,stages};
 }
 stages.credentials.status='passed';stages.setup.status='running';phase='setup';await report();
 await parallel(async row=>{
  const connectors=accounts.get(row.merchant_id);
  for(const name of requirements.connectors)await check(row.merchant_id,'setup',`${name} connector`,async()=>{
   const id=row.connector_accounts[name];if(!id)throw new Error(`No ${name} MCA ID was imported`);
   const account=connectors.find(c=>c.merchant_connector_id===id);
   if(!account)throw new Error('MCA ID was not found for this merchant');
   if(account.connector_name!==name)throw new Error('MCA ID belongs to a different connector');
   if(account.profile_id!==row.profile_id)throw new Error('MCA ID belongs to a different profile');
   if(account.disabled!==false)throw new Error('Connector account is disabled or its enabled state could not be established');
   const methods=account.payment_methods_enabled||[];
   if(!methods.some(m=>m.payment_method==='card'&&m.payment_method_types?.some(t=>t.payment_method_type==='credit'&&(!requirements.recurring||t.recurring_enabled===true))))throw new Error('Connector does not have the required credit-card configuration enabled');
  });
 });
 const ok=!stages.setup.failed;stages.setup.status=ok?'passed':'failed';phase=ok?'saving':'failed';await report();
 return {ok,checks,phase,stages};
}
export function importedCompatibility(set,config){
 const manifest=readJsonSync(path.join(set.path,'manifest.json'),{});
 if(manifest.source!=='imported')return null;
 const merchants=readJsonSync(path.join(set.path,'merchants.json'),[]);
 return {source:'imported',verification:manifest.verification?{ok:manifest.verification.ok,at:manifest.verification.at,failed:manifest.verification.checks.filter(c=>!c.ok).length}:null,compatible:manifest.state==='ready'&&manifest.fingerprint===importFingerprint(config,merchants),connectors:manifest.connectors||[]};
}
export async function prepareImport(profile,input={},existing=null){
 const config={...structuredClone(readJsonSync(configPath)),environment:activeEnvironment(),connectors:profile.connectors,scenarios:profile.scenarios,profile:{id:profile.id}};requireSupportedEnvironment(config.environment);
 if(existing){const before=readJsonSync(path.join(existing.path,'manifest.json'),{});if(before.destination!==config.services.router||before.environment!==config.environment||before.pm!==config.services.modular_pm)throw new Error('The fixture destination changed; reimport its credentials explicitly for the new environment');}
 const id=existing?.id||randomUUID(),directory=existing?.path||path.join(dataRoot,'imported-fixtures',id);
 const merchants=normalizeImportedMerchants(existing?readJsonSync(path.join(directory,'merchants.json')):input.merchants,profile.merchantCount);
 await mkdir(directory,{recursive:true,mode:0o700});
 await atomicJson(path.join(directory,'merchants.json'),merchants);
 await atomicJson(path.join(directory,'manifest.json'),{source:'imported',destination:config.services.router,pm:config.services.modular_pm,environment:config.environment,state:'verifying',profileId:profile.id,connectors:fixtureRequirements(config).connectors});
 if(existing)controlDb().prepare("UPDATE fixture_sets SET status='creating' WHERE id=?").run(id);
 else controlDb().prepare('INSERT INTO fixture_sets VALUES(?,?,?,?,?,?,NULL,?)').run(id,profile.mode,profile.merchantCount,'imported',directory,'creating',Date.now());
 return {id,directory,merchants,config};
}
export async function finishImport(prepared,result){
 const {id,directory,merchants,config}=prepared;
 const sp=config.fixtures.superposition;
 const superposition=sp?{organization_id:sp.organization_id,workspace_id:sp.workspace_id,defaults:sp.defaults}:undefined;
 const merchantsSha=createHash('sha256').update(await readFile(path.join(directory,'merchants.json'))).digest('hex');
 await atomicJson(path.join(directory,'manifest.json'),{schema_version:1,router:config.services.router,merchants_sha256:merchantsSha,merchant_count:merchants.length,superposition,source:'imported',destination:config.services.router,pm:config.services.modular_pm,environment:config.environment,state:result.ok?'ready':'failed',profileId:config.profile?.id,connectors:fixtureRequirements(config).connectors,fingerprint:importFingerprint(config,merchants),verification:{...result,at:Date.now()}});
 controlDb().prepare('UPDATE fixture_sets SET status=? WHERE id=?').run(result.ok?'ready':'failed',id);
}
