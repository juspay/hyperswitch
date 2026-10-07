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
    recorded:progress.recorder.persisted};
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
      if(points.length>3600)points=points.slice(-1800);
    }
  }catch(e){if(e.code!=='ENOENT')throw e;}
  finally{lines.close();stream.destroy();}
  return points.slice(-1800);
}
