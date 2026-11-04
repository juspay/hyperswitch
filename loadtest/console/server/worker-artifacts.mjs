import {createHash,randomUUID} from 'node:crypto';
import {createWriteStream} from 'node:fs';
import {mkdir,readFile,writeFile,rename,rm,stat} from 'node:fs/promises';
import {Readable} from 'node:stream';
import {pipeline} from 'node:stream/promises';
import path from 'node:path';
import {dataRoot,controlDb} from './state.mjs';
import {HttpError} from './errors.mjs';

const identifier=/^[A-Za-z0-9][A-Za-z0-9._:-]{0,199}$/;
const digest=value=>createHash('sha256').update(value).digest('hex');
const bundleDirectory=path.join(dataRoot,'worker-bundles'),artifactDirectory=path.join(dataRoot,'worker-artifacts');
export function workerLoadConfig(config){
  const load=structuredClone(config.load||{});
  if(Array.isArray(load.stages))load.stages=load.stages.map(({fault:_fault,...stage})=>stage);
  return Object.fromEntries(['mode','services','scenarios','payment','sdk','target_headers','thresholds','acceptance','profile','fixtureSet']
    .filter(key=>config[key]!==undefined).map(key=>[key,structuredClone(config[key])]).concat([['load',load]]));
}
export async function createWorkerBundle({runId,config,fixturePath,workerId,segmentStart=0,segmentEnd=1}){
  const [manifest,merchants]=await Promise.all([readFile(path.join(fixturePath,'manifest.json'),'utf8'),readFile(path.join(fixturePath,'merchants.json'),'utf8')]);
  const all=JSON.parse(merchants),start=Math.floor(all.length*segmentStart),end=segmentEnd===1?all.length:Math.floor(all.length*segmentEnd),assigned=all.slice(start,Math.max(start+1,end));
  const fixtureManifest={...JSON.parse(manifest),merchant_count:assigned.length,merchants_sha256:digest(JSON.stringify(assigned))};
  const body=JSON.stringify({schemaVersion:1,runId,workerId,config:workerLoadConfig(config),manifest:fixtureManifest,merchants:assigned}),id=randomUUID();
  await mkdir(bundleDirectory,{recursive:true,mode:0o700});await writeFile(path.join(bundleDirectory,`${id}.json`),body,{mode:0o600,flag:'wx'});
  return {id,sha256:digest(body),bytes:Buffer.byteLength(body)};
}
export async function readWorkerBundle(id,workerId){
  if(!identifier.test(id))throw new Error('Invalid bundle ID');const body=await readFile(path.join(bundleDirectory,`${id}.json`),'utf8');const bundle=JSON.parse(body);
  if(bundle.workerId!==workerId)throw new HttpError(403,'Bundle belongs to another worker');return {body,sha256:digest(body)};
}
export async function storeWorkerArtifact({workerId,runId,name,body,contentLength}){
  if(!identifier.test(runId)||!['results.sqlite','summary.json','progress.json','diagnostics.log'].includes(name))throw new Error('Invalid worker artifact');
  if(!controlDb().prepare('SELECT 1 FROM run_shards WHERE run_id=? AND worker_id=?').get(runId,workerId))throw new HttpError(403,'Worker is not assigned to this run');
  if(contentLength&&contentLength>2_000_000_000)throw new HttpError(413,'Worker artifact exceeds 2 GB');
  const directory=path.join(artifactDirectory,runId,workerId);await mkdir(directory,{recursive:true,mode:0o700});
  const target=path.join(directory,name),temporary=`${target}.${process.pid}.tmp`;let bytes=0;
  const limiter=new TransformStream({transform(chunk,controller){bytes+=chunk.byteLength;if(bytes>2_000_000_000)throw new Error('Worker artifact exceeds 2 GB');controller.enqueue(chunk);}});
  try{await pipeline(Readable.fromWeb(body.pipeThrough(limiter)),createWriteStream(temporary,{mode:0o600,flags:'wx'}));await rename(temporary,target);}catch(error){await rm(temporary,{force:true});throw error;}
  const size=(await stat(target)).size;return {name,size,path:target};
}
export function artifactPath(runId,workerId,name){if(!identifier.test(runId)||!identifier.test(workerId))throw new Error('Invalid artifact identity');return path.join(artifactDirectory,runId,workerId,name);}
