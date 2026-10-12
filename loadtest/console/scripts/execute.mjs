import { readFile,mkdir,writeFile,rename } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const spec=JSON.parse(await readFile(process.argv[2],'utf8'));
const config=JSON.parse(await readFile(spec.configPath,'utf8'));
const {fixtures,load}=await import(pathToFileURL(path.join(spec.loadtestRoot,'k6/scenario-mix/run.mjs')).href);
const fixturePath=spec.fixturePath || path.join(config.output,spec.mode,'fixtures');
const jobDir=path.dirname(process.argv[2]);
process.env.K6_PID_FILE=path.join(jobDir,'k6-pid.json');
process.env.LOADTEST_CONSOLE_JOB='1';
// If the web process dies, stop its generator rather than leaving an orphaned load.
let orphaned=false;
const watchdog=setInterval(()=>{
  try {process.kill(spec.parentPid,0);}catch {
    if(!orphaned){
      orphaned=true;
      setTimeout(async()=>{
        const child=JSON.parse(await readFile(process.env.K6_PID_FILE,'utf8').catch(()=>'{"pid":null}'));
        if(child.pid)try{process.kill(-child.pid,'SIGKILL');}catch{}
        process.exit(1);
      },60000).unref();
      process.kill(process.pid,'SIGTERM');
    }
  }
},1000);
try {
  await mkdir(fixturePath,{recursive:true,mode:0o700});
  let result;
  if(spec.kind==='fixtures') {
    const provision={...config.fixtures,merchant_count:spec.merchantCount ?? (spec.mode==='single'?1:1500),merchant_id_prefix:`${config.fixtures.merchant_id_prefix || 'loadtest'}_${spec.mode}_${spec.fixtureSetId?.slice(0,8) || 'legacy'}`};
    const file=path.join(fixturePath,'provision.json');await writeFile(file,JSON.stringify(provision),{mode:0o600});
    result=await fixtures(file,fixturePath,{onProgress:async setup=>{const progressFile=path.join(jobDir,'fixture-progress.json');await writeFile(progressFile+'.tmp',JSON.stringify(setup),{mode:0o600});await rename(progressFile+'.tmp',progressFile);}});
  } else if(spec.kind==='smoke') {
    const manifest=JSON.parse(await readFile(path.join(fixturePath,'manifest.json'),'utf8'));
    const requested=config.fixtures.superposition;
    if(requested && (!isDeepStrictEqual(requested.defaults,manifest.superposition?.defaults)
      || requested.organization_id!==manifest.superposition?.organization_id || requested.workspace_id!==manifest.superposition?.workspace_id)) throw new Error('Superposition configuration changed; create fixtures again');
    await mkdir(spec.runPath,{recursive:true,mode:0o700});
    const {fixtures:_fixtures,output:_output,...smokeConfig}=config;
    smokeConfig.load={...smokeConfig.load,total_rps:spec.smoke.rps,duration_seconds:spec.smoke.durationSeconds};
    delete smokeConfig.load.stages;delete smokeConfig.load.start_rps;delete smokeConfig.faultPlan;
    const file=path.join(spec.runPath,'config.json');await writeFile(file,JSON.stringify(smokeConfig,null,2),{mode:0o600});
    process.env.SQLITE_RECORDER_CONTROL_FILE=path.join(spec.runPath,'control.json');
    process.env.SQLITE_RECORDER_PROGRESS_FILE=path.join(spec.runPath,'progress.json');
    console.log(`Smoke test · ${spec.smoke.rps} RPS · ${spec.smoke.durationSeconds}s · recording disabled`);
    result=await load(file,fixturePath,spec.runPath,spec.k6Binary,{recording:false,telemetry:true});
  } else {
    const manifest=JSON.parse(await readFile(path.join(fixturePath,'manifest.json'),'utf8'));
    const requested=config.fixtures.superposition;
    if(requested && (!isDeepStrictEqual(requested.defaults,manifest.superposition?.defaults)
      || requested.organization_id!==manifest.superposition?.organization_id || requested.workspace_id!==manifest.superposition?.workspace_id)) throw new Error('Superposition configuration changed; create fixtures again');
    await mkdir(spec.runPath,{recursive:true,mode:0o700});
    const {fixtures:_fixtures,output:_output,...loadConfig}=config;
    const file=path.join(spec.runPath,'config.json');await writeFile(file,JSON.stringify(loadConfig,null,2),{mode:0o600});
    const binary=spec.k6Binary || process.env.K6_BINARY || path.join(spec.loadtestRoot,'k6/scenario-mix/bin/k6-sqlite');
    await writeFile(path.join(jobDir,'phase.json'),JSON.stringify({phase:'load'}),{mode:0o600});
    process.env.SQLITE_RECORDER_CONTROL_FILE=path.join(spec.runPath,'control.json');process.env.SQLITE_RECORDER_PROGRESS_FILE=path.join(spec.runPath,'progress.json');
    result=await load(file,fixturePath,spec.runPath,binary);
  }
  await writeFile(path.join(jobDir,'result.json'),JSON.stringify(result),{mode:0o600});process.exitCode=result.exitCode || 0;
}catch(e) {
  console.error(e.message);await writeFile(path.join(jobDir,'result.json'),JSON.stringify({error:e.message}),{mode:0o600});process.exitCode=1;
}finally{clearInterval(watchdog);}
