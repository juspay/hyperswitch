import {spawn} from 'node:child_process';
import path from 'node:path';
export function validateCommandTarget(target,{requireEnabled=true}={}){
 if(requireEnabled&&process.env.LOADTEST_ALLOW_CUSTOM_COMMANDS!=='true')throw new Error('Custom fault commands are disabled; set LOADTEST_ALLOW_CUSTOM_COMMANDS=true on the supervisor');
 if(!['restore','self-recovering'].includes(target.recoveryMode))throw new Error('Choose explicit restoration or self-recovery');
 if(!['outage','failover'].includes(target.commandAction))throw new Error('Choose outage or failover');
 if(!Number.isInteger(target.timeoutSeconds)||target.timeoutSeconds<1||target.timeoutSeconds>120)throw new Error('Command timeout must be 1–120 seconds');
 if(!path.isAbsolute(target.workingDirectory||''))throw new Error('Use an absolute command working directory');
 for(const field of ['triggerCommand','readyCommand','faultCheckCommand',...(target.recoveryMode==='restore'?['restoreCommand']:[])]){
  if(typeof target[field]!=='string'||!target[field].trim()||target[field].length>8192||target[field].includes('\0'))throw new Error(`Configure ${field}`);
 }
 return target;
}
// Commands intentionally execute on the supervisor, never on distributed agents.
// Output is discarded: arbitrary process output can contain credentials.
export function executeFaultCommand(target,field){
 return new Promise((resolve,reject)=>{
  let child,timeout;
  try{child=spawn('/bin/sh',['-c',target[field]],{cwd:target.workingDirectory,env:process.env,detached:true,stdio:'ignore'});}catch{reject(new Error(`${field} could not start`));return;}
  const kill=()=>{try{process.kill(-child.pid,'SIGKILL');}catch{}};
  timeout=setTimeout(()=>{kill();reject(new Error(`${field} timed out after ${target.timeoutSeconds}s`));},target.timeoutSeconds*1000);
  child.once('error',()=>{clearTimeout(timeout);reject(new Error(`${field} could not start`));});
  child.once('exit',(code,signal)=>{clearTimeout(timeout);if(code===0)resolve();else reject(new Error(`${field} failed (${signal||`exit ${code}`})`));});
 });
}
export function commandFaultAdapter(target,{execute=executeFaultCommand}={}){
 validateCommandTarget(target);
 const succeeds=async field=>{try{await execute(target,field);return true;}catch{return false;}};
 return {
  recoveryTimeoutMs:target.timeoutSeconds*1000,
  probe:async()=>{await execute(target,'readyCommand');return {ready:true};},
  apply:()=>execute(target,'triggerCommand'),
  isDown:()=>succeeds('faultCheckCommand'),
  restore:async()=>{if(target.recoveryMode==='restore')await execute(target,'restoreCommand');},
  verify:()=>succeeds('readyCommand')
 };
}
