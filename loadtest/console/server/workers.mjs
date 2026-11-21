import { Worker } from 'node:worker_threads';
import { workerPath } from './state.mjs';
import { HttpError } from './errors.mjs';
let active=0;
export function queryWorker(task,args={},onProgress=(_value)=>{},onWorker=(_value)=>{}) {
  if(active>=4) return Promise.reject(new HttpError(503,'Server is busy; retry shortly'));
  active++;
  return new Promise((resolve,reject)=>{
    let worker;
    // Test runners and host launchers may add process-only flags that workers reject.
    try {worker=new Worker(workerPath,{workerData:{task,args},execArgv:[]});}
    catch(e){active--;reject(e);return;}
    onWorker(worker);
    let settled=false;
    const finish=(error,result=undefined)=>{if(settled)return;settled=true;active--;error?reject(error):resolve(result);};
    worker.on('message',m=>{if(m.progress)onProgress(m.progress);else finish(m.error?new Error(m.error):null,m.result);});
    worker.once('error',e=>finish(e));worker.once('exit',code=>{if(!settled)finish(new Error(`Worker stopped (${code})`));});
  });
}
