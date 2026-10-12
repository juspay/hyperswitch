import {randomUUID,createHash} from 'node:crypto';
import path from 'node:path';
import {controlDb,publicSettings,validateSettings,readJsonSync,configPath} from './state.mjs';
import {HttpError} from './errors.mjs';
import {loadShape} from '../src/load-shape.mjs';
import {snapshotFaultPlan} from './fault-plan.mjs';
import {faultTargets,localFaultTargets,saveLocalFaultTargets} from './faults.mjs';
const fields=['mode','merchantCount','totalRps','durationSeconds','loadShape','startRps','stagesJson','acceptance','preAllocatedVusMultiplier','maxVusMultiplier','scenarios'];
export function profileDraft(settings=publicSettings()) {
  return Object.fromEntries(fields.map(k=>[k,k==='merchantCount'?(settings.mode==='tenancy'?1500:1):settings[k]]));
}
export function saveProfile(input,id=null) {
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
    db.prepare('INSERT INTO profiles VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,version=excluded.version,updated_at=excluded.updated_at').run(id,name,version,Date.now());
    db.exec('RELEASE profile_save');
  }catch(e){db.exec('ROLLBACK TO profile_save; RELEASE profile_save');throw e;}
  return getProfile(id,version);
}
export function getProfile(id,version=null) {
  const db=controlDb(),row=db.prepare('SELECT * FROM profiles WHERE id=?').get(id);
  if(!row)throw new HttpError(404,'Scenario not found');
  const saved=db.prepare('SELECT body FROM profile_versions WHERE profile_id=? AND version=?').get(id,version ?? row.version);
  if(!saved)throw new HttpError(404,'Scenario version not found');
  return {id,version:version ?? row.version,updatedAt:row.updated_at,...JSON.parse(saved.body)};
}
export function saveScenario({profile,targets,expectedTargets}) {
  const db=controlDb();db.exec('BEGIN IMMEDIATE');
  try {
    if(targets!==undefined){
      if(JSON.stringify(localFaultTargets())!==expectedTargets)throw new HttpError(409,'Shared fault targets changed in another editor. Reopen this scenario before saving.');
      saveLocalFaultTargets(targets);
    }
    const saved=saveProfile(profile,profile.id||null),savedTargets=localFaultTargets();
    db.exec('COMMIT');return {profile:saved,targets:savedTargets};
  }catch(error){db.exec('ROLLBACK');throw error;}
}
export function archiveProfile(id,archived){
  getProfile(id);const db=controlDb();
  if(archived)db.prepare('INSERT OR REPLACE INTO archived_profiles VALUES(?,?)').run(id,Date.now());
  else db.prepare('DELETE FROM archived_profiles WHERE profile_id=?').run(id);
  return {id,archived};
}
export function deleteProfile(id){
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
  return db.prepare('SELECT p.id,a.archived_at FROM profiles p LEFT JOIN archived_profiles a ON a.profile_id=p.id ORDER BY p.updated_at DESC').all().filter(r=>includeArchived||!r.archived_at).map(r=>({...getProfile(r.id),archived:Boolean(r.archived_at)}));
}
function canonical(value){if(Array.isArray(value))return value.map(canonical);if(value&&typeof value==='object')return Object.fromEntries(Object.keys(value).sort().map(k=>[k,canonical(value[k])]));return value;}
export function fixtureSignature(config,mode,count) {
  const sp=config.fixtures?.superposition || {};
  const identity={router:config.fixtures?.router || config.services?.router,connector:config.fixtures?.connector,
    superposition:{organization_id:sp.organization_id,workspace_id:sp.workspace_id,defaults:sp.defaults,context:sp.context},mode,count};
  return createHash('sha256').update(JSON.stringify(canonical(identity))).digest('hex');
}
export function getFixtureSet(id) {
  const row=controlDb().prepare('SELECT * FROM fixture_sets WHERE id=?').get(id);if(!row)throw new HttpError(404,'Fixture set not found');return row;
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
  return db.prepare('SELECT * FROM fixture_sets ORDER BY created_at DESC').all().map(({path:directory,signature,...row})=>{
    const compatible=signature===fixtureSignature(config,row.mode,row.merchant_count);
    const job=row.job_id?db.prepare('SELECT config_path FROM jobs WHERE id=?').get(row.job_id):null;
    const original=job?.config_path?readJsonSync(job.config_path):null;
    const provision=readJsonSync(path.join(directory,'provision.json'),original?.fixtures);
    const before=provision||{},after=config.fixtures||{},changes=[];
    const checks=[['Router',before.router,after.router||config.services?.router],['Connector',before.connector,after.connector],['Superposition organization',before.superposition?.organization_id,after.superposition?.organization_id],['Superposition workspace',before.superposition?.workspace_id,after.superposition?.workspace_id],['Superposition defaults',before.superposition?.defaults,after.superposition?.defaults],['Superposition context',before.superposition?.context,after.superposition?.context]];
    if(!compatible)for(const [label,a,b] of checks)if(JSON.stringify(canonical(a))!==JSON.stringify(canonical(b)))changes.push(label);
    if(!compatible&&!changes.length)changes.push('Saved provisioning configuration');
    return {...row,compatible,changes,destination:before.router||original?.services?.router||'Earlier configuration',label:`${row.mode==='single'?'Single':'Tenancy'} · ${row.merchant_count} merchants`,repairable:compatible&&['failed','interrupted'].includes(row.status)};
  });
}
export function validateFixtureSet(id,profile,config) {
  const set=getFixtureSet(id),manifest=readJsonSync(path.join(set.path,'manifest.json'));
  if(set.status!=='ready'||manifest?.state!=='ready')throw new HttpError(409,'Select a ready fixture set');
  if(set.mode!==profile.mode||set.merchant_count!==profile.merchantCount)throw new HttpError(409,'Fixture merchant mode/count does not match the scenario');
  if(set.signature!==fixtureSignature(config,profile.mode,profile.merchantCount))throw new HttpError(409,'Service or fixture settings changed; create compatible fixtures first');
  return set;
}
