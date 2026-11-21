import {importedCompatibility} from './fixture-import.mjs';
import {connectorSet,normalizeConnectorPlan} from '../src/connector-config.mjs';
import {defaultScenarios,isDefaultScenario} from './default-scenarios.mjs';
import {normalizePaymentFlows} from '../src/payment-flows.mjs';
import {randomUUID,createHash} from 'node:crypto';
import path from 'node:path';
import {controlDb,publicSettings,validateSettings,readJsonSync,configPath,activeEnvironment} from './state.mjs';
import {HttpError} from './errors.mjs';
import {loadShape} from '../src/load-shape.mjs';
import {snapshotFaultPlan} from './fault-plan.mjs';
import {faultTargets,localFaultTargets,saveLocalFaultTargets} from './faults.mjs';
const fields=['connectors','mode','merchantCount','totalRps','durationSeconds','loadShape','startRps','stagesJson','acceptance','preAllocatedVusMultiplier','maxVusMultiplier','scenarios'];
export function profileDraft(settings=publicSettings()) {
  return Object.fromEntries(fields.map(k=>[k,k==='connectors'?normalizeConnectorPlan(settings.connectors):k==='merchantCount'?(settings.mode==='tenancy'?1500:1):settings[k]]));
}
function protectDefault(id){if(isDefaultScenario(id))throw new HttpError(403,'Default scenarios cannot be edited, archived or deleted');}
function builtInProfiles(){return defaultScenarios(activeEnvironment(),faultTargets());}
export function saveProfile(input,id=null) {
  protectDefault(id);
  const db=controlDb(),existing=id?db.prepare('SELECT * FROM profiles WHERE id=?').get(id):null;
  if(id&&!existing)throw new HttpError(404,'Scenario not found');
  if(existing&&input.version!==existing.version)throw new HttpError(409,'Scenario changed; reload before saving');
  const name=String(input.name || '').trim();if(!name||name.length>100)throw new Error('Choose a scenario name (1–100 characters)');
  const merged={...publicSettings(),...profileDraft(input)};
  const count=Number(input.merchantCount ?? (input.mode==='tenancy'?1500:1));
  if(!Number.isInteger(count)||(input.mode==='single'?count!==1:count<2||count>1500))throw new Error('Single needs 1 merchant; Tenancy needs 2–1,500');
  const {config}=validateSettings(merged);
  snapshotFaultPlan(config.load,faultTargets());
  const shape=loadShape(config.load);
  const body={...profileDraft(merged),totalRps:shape.peak,durationSeconds:shape.duration,merchantCount:count,load:config.load,scenarios:config.scenarios,acceptance:config.acceptance};
  id ||=randomUUID();const version=(existing?.version || 0)+1;
  db.exec('SAVEPOINT profile_save');
  try {
    // Recheck inside the transaction so two web processes cannot overwrite each other.
    const latest=db.prepare('SELECT version FROM profiles WHERE id=?').get(id);
    if(latest&&latest.version!==input.version)throw new HttpError(409,'Scenario changed; reload before saving');
    db.prepare('INSERT INTO profile_versions VALUES(?,?,?)').run(id,version,JSON.stringify({...body,name}));
    const now=Date.now();
    db.prepare('INSERT INTO profiles(id,name,version,updated_at,created_at) VALUES(?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,version=excluded.version,updated_at=excluded.updated_at').run(id,name,version,now,now);
    db.exec('RELEASE profile_save');
  }catch(e){db.exec('ROLLBACK TO profile_save; RELEASE profile_save');throw e;}
  return getProfile(id,version);
}
export function getProfile(id,version=null) {
  if(isDefaultScenario(id)){const profile=builtInProfiles().find(p=>p.id===id);if(version!=null&&Number(version)!==profile.version)throw new HttpError(404,'Scenario version not found');return profile;}
  const db=controlDb(),row=db.prepare('SELECT * FROM profiles WHERE id=?').get(id);
  if(!row)throw new HttpError(404,'Scenario not found');
  const saved=db.prepare('SELECT body FROM profile_versions WHERE profile_id=? AND version=?').get(id,version ?? row.version);
  if(!saved)throw new HttpError(404,'Scenario version not found');
  const body=JSON.parse(saved.body);return {id,version:version ?? row.version,updatedAt:row.updated_at,...body,createdAt:row.created_at,connectors:normalizeConnectorPlan(body.connectors),scenarios:normalizePaymentFlows(body.scenarios)};
}
export function saveScenario({profile,targets,expectedTargets}) {
  const db=controlDb();db.exec('BEGIN IMMEDIATE');
  try {
    if(targets!==undefined){
      if(JSON.stringify(localFaultTargets())!==expectedTargets)throw new HttpError(409,'Shared fault targets changed in another editor. Reopen this scenario before saving.');
      saveLocalFaultTargets(targets);
    }
    const fromDefault=isDefaultScenario(profile.id);
    const name=fromDefault&&profile.name===getProfile(profile.id).name?`${profile.name} (copy)`:profile.name;
    const saved=saveProfile({...profile,name},fromDefault?null:profile.id||null),savedTargets=localFaultTargets();
    db.exec('COMMIT');return {profile:saved,targets:savedTargets};
  }catch(error){db.exec('ROLLBACK');throw error;}
}
export function archiveProfile(id,archived){
  protectDefault(id);
  getProfile(id);const db=controlDb();
  if(archived)db.prepare('INSERT OR REPLACE INTO archived_profiles VALUES(?,?)').run(id,Date.now());
  else db.prepare('DELETE FROM archived_profiles WHERE profile_id=?').run(id);
  return {id,archived};
}
export function deleteProfile(id){
  protectDefault(id);
  getProfile(id);const db=controlDb();db.exec('SAVEPOINT profile_delete');
  try{
    db.prepare('DELETE FROM archived_profiles WHERE profile_id=?').run(id);
    db.prepare('DELETE FROM profile_versions WHERE profile_id=?').run(id);
    db.prepare('DELETE FROM profiles WHERE id=?').run(id);
    db.exec('RELEASE profile_delete');return {id,deleted:true};
  }catch(error){db.exec('ROLLBACK TO profile_delete; RELEASE profile_delete');throw error;}
}
export function listProfiles({includeArchived=false}={}) {
  const db=controlDb();
  return [...db.prepare('SELECT p.id,a.archived_at FROM profiles p LEFT JOIN archived_profiles a ON a.profile_id=p.id ORDER BY p.created_at DESC,p.rowid DESC').all().filter(r=>includeArchived||!r.archived_at).map(r=>({...getProfile(r.id),archived:Boolean(r.archived_at)})),...builtInProfiles()];
}
function canonical(value){if(Array.isArray(value))return value.map(canonical);if(value&&typeof value==='object')return Object.fromEntries(Object.keys(value).sort().map(k=>[k,canonical(value[k])]));return value;}
export function fixtureSignature(config,mode,count) {
  const sp=config.fixtures?.superposition || {};
  const identity={environment:config.environment||activeEnvironment(),paymentFlowVersion:2,router:config.fixtures?.router || config.services?.router,connector:config.connectors&&!(config.connectors.mode==='dummy'&&connectorSet(config.connectors).join(',')==='stripe_test')?connectorSet(config.connectors):config.fixtures?.connector,
    superposition:{organization_id:sp.organization_id,workspace_id:sp.workspace_id,defaults:sp.defaults,context:sp.context},mode,count};
  return createHash('sha256').update(JSON.stringify(canonical(identity))).digest('hex');
}
export function getFixtureSet(id) {
  const row=controlDb().prepare("SELECT * FROM fixture_sets WHERE id=? AND status<>'deleted'").get(id);if(!row)throw new HttpError(404,'Fixture set not found');return row;
}
export function deleteFixtureSet(id) {
  const db=controlDb();
  getFixtureSet(id);
  if(db.prepare("SELECT 1 FROM jobs WHERE kind IN ('fixtures','load','smoke') AND status IN ('starting','running','pausing','paused','cancelling') LIMIT 1").get())throw new HttpError(409,'Finish active fixture setup and load tests before deleting fixtures');
  // Keep a tombstone so CLI fixtures are not re-imported on the next refresh.
  // Saved runs and provisioned remote merchants retain their existing data.
  db.prepare("UPDATE fixture_sets SET status='deleted' WHERE id=?").run(id);
  return {id,deleted:true};
}
export function listFixtureSets() {
  const db=controlDb(),config=readJsonSync(configPath);
  // Import existing CLI fixture sets once, without returning their credentials.
  for(const mode of ['single','tenancy']) {
    const directory=path.join(config.output,mode,'fixtures'),manifest=readJsonSync(path.join(directory,'manifest.json'));
    if(manifest?.state==='ready'&&!db.prepare('SELECT id FROM fixture_sets WHERE path=?').get(directory)){
      const provision=readJsonSync(path.join(directory,'provision.json'),config.fixtures);
      const {merchant_count:_count,merchant_id_prefix:_prefix,...original}=provision;
      const imported={...config,fixtures:{...original,merchant_id_prefix:config.fixtures.merchant_id_prefix}};
      const count=manifest.merchant_count || (mode==='single'?1:1500);
      db.prepare('INSERT INTO fixture_sets VALUES(?,?,?,?,?,?,NULL,?)').run(randomUUID(),mode,count,fixtureSignature(imported,mode,count),directory,'ready',Date.parse(manifest.ready_at)||Date.now());
    }
  }
  return db.prepare("SELECT * FROM fixture_sets WHERE status<>'deleted' ORDER BY created_at DESC").all().map(({path:directory,signature,...row})=>{

    const manifest=readJsonSync(path.join(directory,'manifest.json'),{});
    if(manifest.source==='imported'){
      let profile;try{profile=getProfile(manifest.profileId);}catch{}
      const imported=importedCompatibility({...row,path:directory},{...config,environment:activeEnvironment(),connectors:profile?.connectors,scenarios:profile?.scenarios||[]});
      return {...row,...imported,compatible:Boolean(profile)&&imported.compatible,destination:manifest.destination||config.services.router,label:`Imported · ${row.merchant_count} merchants`,repairable:false};
    }
    const job=row.job_id?db.prepare('SELECT config_path FROM jobs WHERE id=?').get(row.job_id):null;
    const original=job?.config_path?readJsonSync(job.config_path):null;
    const provision=readJsonSync(path.join(directory,'provision.json'),original?.fixtures);
    const before=provision||{},after=config.fixtures||{},changes=[];
    const compatible=signature===fixtureSignature(before.connectorPlan?{...config,connectors:before.connectorPlan}:config,row.mode,row.merchant_count);
    const connectors=before.connectorPlan?connectorSet(before.connectorPlan):[before.connector?.connector_name||config.fixtures?.connector?.connector_name||'stripe_test'];
    const checks=[['Router',before.router,after.router||config.services?.router],['Connector',before.connector,after.connector],['Superposition organization',before.superposition?.organization_id,after.superposition?.organization_id],['Superposition workspace',before.superposition?.workspace_id,after.superposition?.workspace_id],['Superposition defaults',before.superposition?.defaults,after.superposition?.defaults],['Superposition context',before.superposition?.context,after.superposition?.context]];
    if(!compatible)for(const [label,a,b] of checks)if(JSON.stringify(canonical(a))!==JSON.stringify(canonical(b)))changes.push(label);
    if(!compatible&&!changes.length)changes.push('Saved provisioning configuration');
    return {...row,source:'created',compatible,connectors,changes,destination:before.router||original?.services?.router||'Earlier configuration',label:`${row.mode==='single'?'Single':'Tenancy'} · ${row.merchant_count} merchants`,repairable:compatible&&['failed','interrupted'].includes(row.status)};
  }).filter(row=>row.source!=='imported'||row.status==='ready');
}
export function validateFixtureSet(id,profile,config) {
  const set=getFixtureSet(id),manifest=readJsonSync(path.join(set.path,'manifest.json'));
  if(set.status!=='ready'||manifest?.state!=='ready')throw new HttpError(409,'Select a ready fixture set');
  if(set.mode!==profile.mode||set.merchant_count!==profile.merchantCount)throw new HttpError(409,'Fixture merchant mode/count does not match the scenario');
  const imported=importedCompatibility(set,config);
  if(imported){if(!imported.compatible)throw new HttpError(409,'Imported fixture verification is stale or failed; verify it for this scenario again');return set;}
  if(set.signature!==fixtureSignature(config,profile.mode,profile.merchantCount))throw new HttpError(409,'Service or fixture settings changed; create compatible fixtures first');
  return set;
}
