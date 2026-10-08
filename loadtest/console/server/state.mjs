import { DatabaseSync } from 'node:sqlite';
import { mkdirSync, readFileSync, writeFileSync, chmodSync } from 'node:fs';
import { readFile, writeFile, rename } from 'node:fs/promises';
import path from 'node:path';
import {loadShape} from '../src/load-shape.mjs';
import {defaultCriteria} from './acceptance.mjs';
import {decodeStages,encodeStages} from '../src/ramp-editor.mjs';

export const consoleRoot = path.resolve(process.env.LOADTEST_CONSOLE_ROOT || process.cwd());
export const loadtestRoot = path.resolve(process.env.LOADTEST_ROOT || path.join(consoleRoot, '..'));
export const dataRoot = path.resolve(process.env.LOADTEST_DATA_DIR || path.join(consoleRoot, '.data'));
export const configPath = path.join(dataRoot, 'config.json');
export const localPath = path.join(dataRoot, 'settings.json');
export const workerPath = path.join(consoleRoot, 'server', 'query-worker.mjs');
let database;

export function controlDb() {
  if (database) return database;
  mkdirSync(dataRoot, { recursive: true, mode: 0o700 });
  database = new DatabaseSync(path.join(dataRoot, 'console.sqlite'));
  chmodSync(path.join(dataRoot, 'console.sqlite'), 0o600);
  database.exec(`PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;
    CREATE TABLE IF NOT EXISTS profiles(id TEXT PRIMARY KEY,name TEXT NOT NULL,version INTEGER NOT NULL,updated_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS profile_versions(profile_id TEXT NOT NULL,version INTEGER NOT NULL,body TEXT NOT NULL,PRIMARY KEY(profile_id,version));
    CREATE TABLE IF NOT EXISTS fixture_sets(id TEXT PRIMARY KEY,mode TEXT NOT NULL,merchant_count INTEGER NOT NULL,signature TEXT NOT NULL,path TEXT NOT NULL,status TEXT NOT NULL,job_id TEXT,created_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS sessions(token TEXT PRIMARY KEY,expires INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS login_limits(key TEXT PRIMARY KEY,failures INTEGER NOT NULL,reset_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS jobs(id TEXT PRIMARY KEY,kind TEXT NOT NULL,mode TEXT,status TEXT NOT NULL,
      started_at INTEGER NOT NULL,ended_at INTEGER,exit_code INTEGER,run_path TEXT,config_path TEXT,
      output TEXT NOT NULL DEFAULT '',progress TEXT NOT NULL DEFAULT '{}',result TEXT,error TEXT);
    CREATE TABLE IF NOT EXISTS job_events(event_id INTEGER PRIMARY KEY AUTOINCREMENT,job_id TEXT NOT NULL,at INTEGER NOT NULL,kind TEXT NOT NULL,message TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS events_job ON job_events(job_id,event_id);
    DROP INDEX IF EXISTS single_generator;
    CREATE UNIQUE INDEX single_generator ON jobs((1))
      WHERE kind IN ('fixtures','load') AND status IN ('starting','running','pausing','paused','cancelling');`);
  return database;
}

export function readJsonSync(file, fallback = null) {
  try { return JSON.parse(readFileSync(file, 'utf8')); }
  catch (e) { if (e.code === 'ENOENT') return fallback; throw e; }
}
export async function readJson(file, fallback = null) {
  try { return JSON.parse(await readFile(file, 'utf8')); }
  catch (e) { if (e.code === 'ENOENT') return fallback; throw e; }
}
export async function atomicJson(file, value) {
  const temporary = `${file}.${process.pid}.tmp`;
  await writeFile(temporary, JSON.stringify(value, null, 2) + '\n', { mode: 0o600 });
  await rename(temporary, file);
}

export function initializeConfig() {
  mkdirSync(dataRoot, { recursive: true, mode: 0o700 });
  if (!readJsonSync(configPath)) {
    const source = path.resolve(process.env.LOADTEST_CONFIG_FILE || path.join(loadtestRoot, 'loadtest.config.json'));
    const legacy = readJsonSync(source);
    const config = legacy || readJsonSync(path.join(loadtestRoot, 'loadtest.config.example.json'));
    if (!config) throw new Error('Load-test configuration is missing');
    config.output = legacy ? path.resolve(path.dirname(source), config.output || 'artifacts') : path.join(dataRoot, 'artifacts');
    writeFileSync(configPath, JSON.stringify(config, null, 2), { mode: 0o600 });
  }
}

export function localSettings() {
  return {
    mode: 'single', grafana: { baseUrl: process.env.GRAFANA_URL || 'http://localhost:3002', panels: [] },
    database: { host: process.env.PGHOST || '127.0.0.1', port: Number(process.env.PGPORT || 5432),
      database: process.env.PGDATABASE || 'hyperswitch_db', user: process.env.PGUSER || '', password: process.env.PGPASSWORD || '', ssl: false },
    ...readJsonSync(localPath, readJsonSync(path.join(consoleRoot, '.console-state.json'), {}))
  };
}

export function publicSettings() {
  initializeConfig();
  const c = readJsonSync(configPath), l = localSettings(), sp = c.fixtures?.superposition || {};
  return {
    mode: l.mode, router: c.services.router, modularPm: c.services.modular_pm,
    adminApiKey: '', hasAdminApiKey: Boolean(c.fixtures.admin_api_key), connector: c.fixtures.connector?.connector_name || 'stripe_test',
    totalRps: loadShape(c.load).peak, durationSeconds: loadShape(c.load).duration,
    loadShape:c.load.stages?'staged':'flat',startRps:c.load.start_rps ?? c.load.stages?.[0]?.target_rps ?? 10,stagesJson:JSON.stringify(c.load.stages || [{target_rps:100,duration_seconds:60},{target_rps:500,duration_seconds:60},{target_rps:1000,duration_seconds:1800}],null,2),
    acceptance:{...defaultCriteria,...c.acceptance},
    preAllocatedVusMultiplier: c.load.pre_allocated_vus_multiplier, maxVusMultiplier: c.load.max_vus_multiplier,
    scenarios: c.scenarios,
    superposition: { endpoint: sp.endpoint, token: '', hasToken: Boolean(sp.token), organizationId: sp.organization_id,
      workspaceId: sp.workspace_id, refreshSeconds: sp.refresh_interval_seconds ?? 30, defaultsJson: JSON.stringify(sp.defaults || {}, null, 2) },
    grafana: { baseUrl: l.grafana.baseUrl, panels: l.grafana.panels || [], hasToken: Boolean(process.env.GRAFANA_TOKEN), hasIdentity: Boolean(process.env.GRAFANA_TOKEN || process.env.GRAFANA_USER) },
    database: { host: l.database.host, port: l.database.port, database: l.database.database, user: l.database.user,
      ssl: Boolean(l.database.ssl), password: '', hasPassword: Boolean(l.database.password) }
  };
}

export const flows = ['guest','cit_on_session','cit_off_session','ptv_on_session','ptv_off_session','cit_metadata_changed','sdk_checkout','mit','saved_card_checkout'];
export function httpUrl(value) {
  const u = new URL(value);
  if (!['http:', 'https:'].includes(u.protocol) || u.username || u.password) throw new Error('Use an HTTP(S) URL without embedded credentials');
  return u;
}
export function panelPath(value, base) {
  const u = httpUrl(new URL(value, httpUrl(base)).href);
  if (u.origin !== new URL(base).origin) throw new Error('Set Settings → Grafana → Base URL to this Grafana server before adding its panels');
  const route = u.pathname.match(/^\/(?:grafana\/)?(d|d-solo)\/(.+)$/);
  if (!route) throw new Error('Paste a Grafana panel embed URL or a dashboard URL with a selected panel');
  const panel = u.searchParams.get('panelId') || u.searchParams.get('viewPanel');
  if (!panel) throw new Error('Select a panel in Grafana, then use Share → Embed. A dashboard URL alone does not identify the panel');
  u.pathname = '/grafana/d-solo/' + route[2];
  u.searchParams.set('panelId', panel);
  u.searchParams.delete('viewPanel');
  return u.pathname + u.search;
}

export function validateSettings(input) {
  const current = readJsonSync(configPath), local = localSettings();
  if(input.loadShape!=null&&!['flat','staged'].includes(input.loadShape))throw new Error('Invalid load shape');
  if (!['single','tenancy'].includes(input.mode)) throw new Error('Invalid merchant mode');
  for (const value of [input.router,input.modularPm,input.superposition.endpoint,input.grafana.baseUrl]) httpUrl(value);
  if (!Array.isArray(input.scenarios) || !input.scenarios.length || input.scenarios.length > 50) throw new Error('Choose 1–50 flows');
  const names = new Set();
  for (const f of input.scenarios) {
    if (!/^[A-Za-z0-9_]{1,80}$/.test(f.name) || names.has(f.name) || !flows.includes(f.scenario)
      || !['modular','non_modular'].includes(f.merchant_path) || !Number.isFinite(Number(f.weight)) || Number(f.weight) < 0 || Number(f.weight) > 100) throw new Error('Invalid or duplicate flow');
    if ((f.scenario.startsWith('ptv_') && f.merchant_path !== 'modular')
      || (['cit_metadata_changed','sdk_checkout','mit','saved_card_checkout'].includes(f.scenario) && f.merchant_path !== 'non_modular')) throw new Error('Unsupported merchant path for flow');
    names.add(f.name);
  }
  if (input.scenarios.reduce((s,f)=>s+Number(f.weight),0) !== 100) throw new Error('Flow weights must total 100%');
  for (const v of [input.totalRps,input.durationSeconds]) if (!Number.isSafeInteger(Number(v)) || v < 1) throw new Error('Rate and duration must be positive integers');
  const defaults = JSON.parse(input.superposition.defaultsJson);
  if (!defaults || typeof defaults !== 'object' || Array.isArray(defaults)) throw new Error('Defaults must be an object');
  const refresh = Number(input.superposition.refreshSeconds);
  if (!Number.isFinite(refresh) || refresh < 0) throw new Error('Invalid refresh wait');
  const pre = Number(input.preAllocatedVusMultiplier ?? current.load.pre_allocated_vus_multiplier);
  const max = Number(input.maxVusMultiplier ?? current.load.max_vus_multiplier);
  if (!(pre > 0 && max >= pre)) throw new Error('VU multipliers must satisfy 0 < preallocated <= max');
  const dbPort = Number(input.database.port);
  if (!Number.isInteger(dbPort) || dbPort < 1 || dbPort > 65535) throw new Error('Invalid database port');
  const panels = (input.grafana.panels || []).map(p=>({ title: String(p.title).slice(0,100), url: panelPath(p.url,input.grafana.baseUrl) }));
  const next = structuredClone(current);
  next.services = { ...next.services,router: input.router,modular_pm:input.modularPm };
  next.load = { ...next.load,total_rps:Number(input.totalRps),duration_seconds:Number(input.durationSeconds),pre_allocated_vus_multiplier:pre,max_vus_multiplier:max };
  delete next.load.phases;
  delete next.load.stages;delete next.load.start_rps;
  if(input.loadShape==='staged'){
    const stages=JSON.parse(input.stagesJson);
    if(!Array.isArray(stages)||stages.length<1||stages.length>50||stages.some(s=>!Number.isFinite(s.target_rps)||s.target_rps<=0||!Number.isInteger(s.duration_seconds)||s.duration_seconds<1))throw new Error('Choose 1–50 positive TPS stages with integer durations');
    const start=Number(input.startRps);if(!Number.isFinite(start)||start<0)throw new Error('Starting TPS must be nonnegative');
    next.load.stages=encodeStages(decodeStages(JSON.stringify(stages),start),start);next.load.start_rps=start;delete next.load.total_rps;delete next.load.duration_seconds;
  }
  const criteria={...defaultCriteria,...input.acceptance};
  for(const v of Object.values(criteria))if(!Number.isFinite(Number(v))||Number(v)<0)throw new Error('Invalid acceptance criteria');
  if(criteria.minimumSuccessRatio>1||criteria.maximumErrorPercent>100)throw new Error('Invalid acceptance percentage');
  next.acceptance=Object.fromEntries(Object.keys(defaultCriteria).map(k=>[k,Number(criteria[k])]));
  next.scenarios = input.scenarios.map(f=>({name:f.name,scenario:f.scenario,merchant_path:f.merchant_path,weight:Number(f.weight)}));
  next.fixtures.router = input.router;
  if (input.adminApiKey) next.fixtures.admin_api_key = input.adminApiKey;
  next.fixtures.connector.connector_name = input.connector;
  next.fixtures.superposition = { ...next.fixtures.superposition,endpoint:input.superposition.endpoint,
    organization_id:input.superposition.organizationId,workspace_id:input.superposition.workspaceId,defaults,refresh_interval_seconds:refresh };
  if (input.superposition.token) next.fixtures.superposition.token = input.superposition.token;
  return {config:next,local:{ mode:input.mode,grafana:{baseUrl:input.grafana.baseUrl,panels}, database:{
    host:input.database.host,port:dbPort,database:input.database.database,user:input.database.user,ssl:Boolean(input.database.ssl),
    password:input.database.password || local.database.password } }};
}

export async function saveSettings(input) {
  initializeConfig();
  const {config,local}=validateSettings(input);
  await atomicJson(configPath,config);await atomicJson(localPath,local);
  return publicSettings();
}

export async function saveConnections(input) {
  // Connection edits never depend on an unsaved profile in another screen.
  const current=publicSettings();
  for(const key of ["router","modularPm","adminApiKey","connector","superposition","grafana","database"])if(input[key]!==undefined)current[key]=input[key];
  return saveSettings(current);
}

export function runDirectory(id) {
  const match = /^(single|tenancy):([A-Za-z0-9][A-Za-z0-9._-]*)$/.exec(id);
  if (!match) throw new Error('Invalid run ID');
  const c = readJsonSync(configPath);
  return { mode:match[1], name:match[2], path:path.join(c.output,match[1],'runs',match[2]) };
}

export function redact(value) {
  let text = String(value);
  const c = readJsonSync(configPath,{fixtures:{}}), l = localSettings();
  const secrets = [c.fixtures.admin_api_key,c.fixtures.superposition?.token,l.database.password,process.env.GRAFANA_TOKEN,process.env.ROUTER_ADMIN_API_KEY,process.env.SUPERPOSITION_TOKEN];
  for (let s of secrets) { if (typeof s === 'string' && s.startsWith('${')) s = process.env[s.slice(2,-1)]; if(s) text=text.replaceAll(s,'[redacted]'); }
  return text;
}
