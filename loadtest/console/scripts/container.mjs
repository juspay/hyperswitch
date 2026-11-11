// Container shutdown owns both services; restarting SSR alone leaves jobs alive.
import {spawn} from 'node:child_process';
import {initializeSupervisor} from '../server/supervisor.mjs';
const service=await initializeSupervisor();
const web=spawn(process.execPath,['start.mjs'],{stdio:'inherit',env:process.env});
let stopping=false;
async function stop(signal='SIGTERM'){
  if(stopping)return;stopping=true;
  web.kill(signal);try{process.kill(service.pid,'SIGTERM');}catch{}
  const deadline=Date.now()+70000;
  while(Date.now()<deadline){try{process.kill(service.pid,0);}catch{process.exit(0);}await new Promise(r=>setTimeout(r,200));}
  try{process.kill(service.pid,'SIGKILL');}catch{}process.exit(1);
}
process.on('SIGTERM',()=>stop());process.on('SIGINT',()=>stop());
web.on('exit',()=>{if(!stopping)stop();});
