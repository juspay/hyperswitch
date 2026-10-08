import {randomUUID,createHash} from 'node:crypto';
import path from 'node:path';
import {controlDb,publicSettings,validateSettings,readJsonSync,configPath} from './state.mjs';
import {HttpError} from './errors.mjs';
import {loadShape} from '../src/load-shape.mjs';
import {snapshotFaultPlan} from './fault-plan.mjs';
import {faultTargets} from './faults.mjs';
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
  db.exec('BEGIN IMMEDIATE');
  try {
    // Recheck inside the transaction so two web processes cannot overwrite each other.
    const latest=db.prepare('SELECT version FROM profiles WHERE id=?').get(id);
    if(latest&&latest.version!==input.version)throw new HttpError(409,'Scenario changed; reload before saving');
    db.prepare('INSERT INTO profile_versions VALUES(?,?,?)').run(id,version,JSON.stringify({...body,name}));
    db.prepare('INSERT INTO profiles VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,version=excluded.version,updated_at=excluded.updated_at').run(id,name,version,Date.now());
    db.exec('COMMIT');
  }catch(e){db.exec('ROLLBACK');throw e;}
  return getProfile(id,version);
}
export function getProfile(id,version=null) {
  const db=controlDb(),row=db.prepare('SELECT * FROM profiles WHERE id=?').get(id);
  if(!row)throw new HttpError(404,'Scenario not found');
  const saved=db.prepare('SELECT body FROM profile_versions WHERE profile_id=? AND version=?').get(id,version ?? row.version);
  if(!saved)throw new HttpError(404,'Scenario version not found');
  return {id,version:version ?? row.version,updatedAt:row.updated_at,...JSON.parse(saved.body)};
}
export function listProfiles() {
  const db=controlDb();
  if(!db.prepare('SELECT id FROM profiles LIMIT 1').get())saveProfile({...profileDraft(),name:'Default scenario'});
  return db.prepare('SELECT id FROM profiles ORDER BY updated_at DESC').all().map(r=>getProfile(r.id));
}
function canonical(value){if(Array.isArray(value))return value.map(canonical);if(value&&typeof value==='object')return Object.fromEntries(Object.keys(value).sort().map(k=>[k,canonical(value[k])]));return value;}
export function fixtureSignature(config,mode,count) {
  return createHash('sha256').update(JSON.stringify(canonical({fixtures:config.fixtures,services:config.services,mode,count}))).digest('hex');
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
  return db.prepare('SELECT * FROM fixture_sets ORDER BY created_at DESC').all().map(({path:_path,signature,...row})=>({...row,compatible:signature===fixtureSignature(config,row.mode,row.merchant_count)}));
}
export function validateFixtureSet(id,profile,config) {
  const set=getFixtureSet(id),manifest=readJsonSync(path.join(set.path,'manifest.json'));
  if(set.status!=='ready'||manifest?.state!=='ready')throw new HttpError(409,'Select a ready fixture set');
  if(set.mode!==profile.mode||set.merchant_count!==profile.merchantCount)throw new HttpError(409,'Fixture merchant mode/count does not match the scenario');
  if(set.signature!==fixtureSignature(config,profile.mode,profile.merchantCount))throw new HttpError(409,'Service or fixture settings changed; create compatible fixtures first');
  return set;
}
