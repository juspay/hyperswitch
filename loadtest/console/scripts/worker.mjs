import {readJson,dataRoot} from '../server/state.mjs';
import {initializeSupervisor} from '../server/supervisor.mjs';
import path from 'node:path';
const command=process.argv[2] || 'status';
if(command==='start'){const service=await initializeSupervisor();console.log(`Worker ready · PID ${service.pid}`);}
else {
  const service=await readJson(path.join(dataRoot,'worker-service.json'),null);
  if(command==='stop'){
    if(service){try{process.kill(service.pid,'SIGTERM');console.log('Worker stopping · active jobs drain and stop');}catch(e){if(e.code!=='ESRCH')throw e;}}
    else console.log('Worker is stopped');
  }else if(command==='status'){
    let running=false;if(service)try{process.kill(service.pid,0);running=true;}catch{}
    console.log(running?`Worker running · PID ${service.pid}`:'Worker is stopped');
  }else throw Error('Use start, status or stop');
}
