import {access,readFile} from 'node:fs/promises';
import {constants} from 'node:fs';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import path from 'node:path';

const mode=process.argv[2];
if(!['dev','start'].includes(mode))throw new Error('Use database-tls.mjs dev or start');
process.env.LOADTEST_DB_TLS='1';
if(process.env.PGSSLROOTCERT){
  const certificate=path.resolve(process.env.PGSSLROOTCERT);
  await access(certificate,constants.R_OK);
  process.env.PGSSLROOTCERT=certificate;
}

// A detached worker inherits its startup environment. Reload it only while idle
// so reconciliation uses the requested CA without interrupting load generation.
const {dataRoot,consoleRoot}=await import('../server/state.mjs');
const descriptorPath=path.join(dataRoot,'worker-service.json');
let descriptor=null;
try{descriptor=JSON.parse(await readFile(descriptorPath,'utf8'));}catch{}
let workerAlive=false;
if(descriptor)try{process.kill(descriptor.pid,0);workerAlive=true;}catch(e){if(e.code!=='ESRCH')throw e;}
if(descriptor&&workerAlive){
  const rpc=async(operation)=>{
    const response=await fetch(`${descriptor.url}/rpc`,{method:'POST',headers:{authorization:`Bearer ${descriptor.token}`,'content-type':'application/json'},body:JSON.stringify({operation,args:{}}),signal:AbortSignal.timeout(3000)});
    if(!response.ok)throw new Error(`Worker ${operation} check failed`);return response.json();
  };
  try{
    const [active,faults]=await Promise.all([rpc('active'),rpc('faults')]);
    if(active||faults.active||faults.blocked||faults.recovering)throw new Error('Cannot enable database TLS while a job or fault recovery is active');
    process.kill(descriptor.pid,'SIGTERM');
    for(let i=0;i<100;i++){try{process.kill(descriptor.pid,0);await new Promise(resolve=>setTimeout(resolve,100));}catch{workerAlive=false;break;}}
    if(workerAlive)throw new Error('Timed out while reloading the idle worker for database TLS');
  }catch(e){if(e.code!=='ESRCH')throw e;}
}
const {initializeSupervisor}=await import('../server/supervisor.mjs');
await initializeSupervisor();

const entry=mode==='dev'?path.join(consoleRoot,'node_modules/vite/bin/vite.js'):path.join(consoleRoot,'start.mjs');
const args=mode==='dev'?[entry,'--host',process.env.HOST || '127.0.0.1','--port',process.env.PORT || '4174']:[entry];
const child=spawn(process.execPath,args,{cwd:consoleRoot,stdio:'inherit',env:process.env});
const [code]=await once(child,'exit');
process.exitCode=code ?? 1;
