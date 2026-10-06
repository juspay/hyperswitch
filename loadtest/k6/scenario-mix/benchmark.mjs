#!/usr/bin/env node
// Local characterization only: does not contact Router, Superposition, or any connector.
import { readFile,writeFile,mkdir } from 'node:fs/promises';
import { DatabaseSync } from 'node:sqlite';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { mockServer } from './test/mock-server.mjs';
import { fixtures,load } from './run.mjs';

const root=path.dirname(fileURLToPath(import.meta.url));
const flags={rps:2000,seconds:30,output:'runs/mock-benchmark',polls:1};
for(let i=2;i<process.argv.length;i+=2) {
 const key=process.argv[i].replace(/^--/,'');
 if(!Object.hasOwn(flags,key) || !process.argv[i+1]) throw new Error(`Unknown/missing flag ${process.argv[i]}`);
 flags[key]=key==='output'?process.argv[i+1]:Number(process.argv[i+1]);
}
if(!Number.isFinite(flags.rps)||flags.rps<=0||!Number.isFinite(flags.seconds)||flags.seconds<1||!Number.isSafeInteger(flags.polls)||flags.polls<0||flags.polls>49) throw new Error('Invalid benchmark rps/seconds/polls');
const output=path.resolve(flags.output);await mkdir(output,{recursive:true,mode:0o700});
const {server,state,url}=await mockServer({baselinePolls:flags.polls,listPolls:flags.polls});
try {
 const provisioning={router:url,admin_api_key:'mock-admin',merchant_count:1,merchant_id_prefix:'mock',concurrency:1,retry:{attempts:1,backoff_ms:0},connector:{connector_account_details:{auth_type:'HeaderKey',api_key:'mock-key'}}};
 const provisionFile=path.join(output,'provision.json');await writeFile(provisionFile,JSON.stringify(provisioning));
 const fixtureOutput=path.join(output,'fixtures');await fixtures(provisionFile,fixtureOutput);
 const config=JSON.parse(await readFile(path.join(root,'config.example.json'),'utf8'));
 config.services={router:url,modular_pm:`${url}/v2`};config.thresholds={};
 config.load={total_rps:flags.rps,duration_seconds:flags.seconds,pre_allocated_vus_multiplier:0.5,max_vus_multiplier:2,request_timeout_ms:30000};
 const configFile=path.join(output,'load.json');await writeFile(configFile,JSON.stringify(config));
 const report={kind:'local_mock',offered_flows_per_second:flags.rps,duration_seconds:flags.seconds,poll_delay_attempts:flags.polls};
 for(const recording of [false,true]) {
  const name=recording?'recorded':'unrecorded',runOutput=path.join(output,name),before=state.requests;
  const result=await load(configFile,fixtureOutput,runOutput,path.join(root,'bin/k6-sqlite'),{recording,timing:true});
  const summary=JSON.parse(await readFile(path.join(runOutput,'summary.json'),'utf8'));
  const count=(metric)=>summary.metrics[metric]?.values?.count||0;
  const successes=Object.entries(summary.metrics).filter(([key])=>key.startsWith('scenario_success_')).reduce((n,[,metric])=>n+(metric.values.count||0),0);
  report[name]={exit_code:result.exitCode,cpu_seconds:(result.cpu?.user||0)+(result.cpu?.sys||0),
   offered_flows_per_second:flags.rps,achieved_successful_tps:successes/flags.seconds,
   iterations:count('iterations'),dropped_iterations:count('dropped_iterations'),http_attempts:count('http_reqs'),http_responses:state.requests-before};
  if(recording) {
   const db=new DatabaseSync(result.database,{readOnly:true});
   const run=db.prepare('SELECT * FROM runs').get();
   const events=db.prepare('SELECT COUNT(*) n FROM request_events').get().n;
   report[name].recorder=JSON.parse(run.metadata).recorder;
   report[name].recording_gaps=report[name].http_attempts-events;
   const responses=db.prepare('SELECT COUNT(*) n FROM request_events WHERE api_status_code>0').get().n;
   report[name].response_gaps=report[name].http_responses-responses;
   report[name].transport_failures=db.prepare('SELECT COUNT(*) n FROM request_events WHERE api_status_code=0').get().n;
   db.close();
  }
 }
 report.cpu_overhead_percent=report.unrecorded.cpu_seconds ? (report.recorded.cpu_seconds/report.unrecorded.cpu_seconds-1)*100 : null;
 report.acceptance_passed=report.recorded.exit_code===0 && report.recorded.recording_gaps===0 && report.recorded.response_gaps===0 && report.recorded.dropped_iterations===0 && report.unrecorded.dropped_iterations===0 && report.cpu_overhead_percent!==null && report.cpu_overhead_percent<5;
 await writeFile(path.join(output,'benchmark.json'),JSON.stringify(report,null,2));
 console.log(JSON.stringify(report,null,2));
 if(!report.acceptance_passed) process.exitCode=2;
} finally {server.close();}
