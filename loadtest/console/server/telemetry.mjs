import {compactSamples} from '../src/chart-timeline.mjs';
import { appendFile } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { createInterface } from 'node:readline';
import path from 'node:path';

export function telemetryPoint(progress,previous=null) {
  if(!Number.isFinite(progress.elapsed_seconds))return null;
  const successes=Object.values(progress.flows || {}).reduce((sum,flow)=>sum+flow.successes,0);
  const interval=progress.elapsed_seconds-(previous?.t || 0);
  // Use counter deltas over the displayed interval, including the final partial sample.
  return {t:progress.elapsed_seconds,successes,tps:interval>0?(successes-(previous?.successes || 0))/interval:0,
    errors:progress.http_attempts?100*progress.http_errors/progress.http_attempts:0,
    recorded:progress.recorder.persisted,p99:progress.p99_ms ?? null,activeSeconds:progress.control?.active_seconds ?? progress.elapsed_seconds,
    state:progress.control?.state || 'running',offeredTps:progress.control?.offered_tps ?? null,
    inFlight:progress.control?.in_flight ?? null,backlog:progress.recorder.enqueued-progress.recorder.persisted,
    flows:Object.fromEntries(Object.entries(progress.flows || {}).map(([name,f])=>[name,{successes:f.successes,failures:f.failures,p90:f.p90_ms,p99:f.p99_ms,tps:interval>0?(f.successes-(previous?.flows?.[name]?.successes || 0))/interval:0}]))};
}
export async function appendTelemetry(directory,point) {
  if(point)await appendFile(path.join(directory,'telemetry.jsonl'),JSON.stringify(point)+'\n',{mode:0o600});
}
export async function readTelemetry(directory) {
  const stream=createReadStream(path.join(directory,'telemetry.jsonl'),{encoding:'utf8'});
  const lines=createInterface({input:stream,crlfDelay:Infinity});let points=[];
  try {
    for await(const line of lines){
      try {const point=JSON.parse(line);if(Number.isFinite(point.t))points.push(point);}catch{/* Ignore an interrupted trailing write. */}
      if(points.length>3600)points=compactSamples(points);
    }
  }catch(e){if(e.code!=='ENOENT')throw e;}
  finally{lines.close();stream.destroy();}
  return compactSamples(points);
}
