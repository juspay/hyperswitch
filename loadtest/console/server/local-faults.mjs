import {spawn} from 'node:child_process';
import {open,access,realpath} from 'node:fs/promises';
import {constants} from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import {dataRoot,atomicJson,readJson} from './state.mjs';
export function reachable(port){return new Promise(resolve=>{const socket=net.connect({host:'127.0.0.1',port});const done=value=>{socket.destroy();resolve(value);};socket.setTimeout(1000,()=>done(false));socket.once('connect',()=>done(true));socket.once('error',()=>done(false));});}
const pause=ms=>new Promise(r=>setTimeout(r,ms));
export async function nixExecutable(){
 for(const binary of [...(process.env.PATH || '').split(path.delimiter).filter(Boolean).map(directory=>path.join(directory,'nix')),'/nix/var/nix/profiles/default/bin/nix']){
  try{await access(binary,constants.X_OK);return binary;}catch{}
 }
 throw new Error('Nix is unavailable; card vault cannot be recovered. Add nix to the worker PATH before creating a fault.');
}
export async function vaultLaunchSpec(target){
 await access(path.join(target.repository,'Cargo.toml'));
 try{await access(path.join(target.repository,'flake.nix'));return {binary:await nixExecutable(),args:['develop','-c','cargo','r']};}catch(e){if(e.code!=='ENOENT')throw e;}
 for(const mode of ['debug','release']){const binary=path.join(target.repository,'target',mode,'locker');try{await access(binary,constants.X_OK);return {binary,args:[]};}catch{}}
 throw new Error('No vault flake.nix or executable target/debug/locker (or target/release/locker). Build the vault before testing an outage.');
}
export function processComposeController(target,request=fetch){
 const origin=`http://127.0.0.1:${target.controllerPort}`;
 const call=async(pathname,method='GET')=>{
  let response;
  try{response=await request(`${origin}${pathname}`,{method,signal:AbortSignal.timeout(5000)});}
  catch(error){throw new Error(`Cannot reach process-compose controller at ${origin}: ${error.cause?.code||error.message}`);}
  const body=await response.text();
  if(!response.ok){const detail=body.trim();throw new Error(response.status===404?`Process ${target.process} is not configured in process-compose`:`Process-compose ${method} ${pathname} failed (${response.status})${detail?`: ${detail}`:''}`);}
  if(!body.trim())return null;
  try{return JSON.parse(body);}catch{throw new Error(`Process-compose returned invalid JSON for ${pathname}`);}
 };
 const name=encodeURIComponent(target.process);
 return {inspect:()=>call(`/process/${name}`),list:async()=>{const result=await call('/processes');return Array.isArray(result)?result:result?.data;},stop:()=>call(`/process/stop/${name}`,'PATCH'),start:()=>call(`/process/start/${name}`,'POST')};
}
export function localAdapter(t,command,hooks={}){
 const up=hooks.reachable||reachable;
 if(t.adapter==='process-compose'){
  const controller=hooks.controller;
  const flags=['-p',String(t.controllerPort),'process','-a','127.0.0.1'];
  const inspect=async()=>{if(controller)return controller.inspect();try{const result=JSON.parse(await command(t.binary,[...flags,'get',t.process,'-o','json']));const info=Array.isArray(result)?result.find(p=>p.name===t.process):result;if(!info)throw new Error('Missing process');return info;}catch(error){const reason=error.code==='ENOENT'?`process-compose binary not found: ${t.binary}`:error.message;throw new Error(`Cannot query process-compose at 127.0.0.1:${t.controllerPort} for ${t.process}: ${reason}`);}};
  const running=info=>String(info.status).toLowerCase()==='running';
  const identity=info=>({name:info.name,command:info.command,namespace:info.namespace});
  const same=(info,before)=>JSON.stringify(identity(info))===JSON.stringify(before.identity);
  const list=async()=>controller?controller.list():JSON.parse(await command(t.binary,['-p',String(t.controllerPort),'process','list','-o','json']));
  const stop=()=>controller?controller.stop():command(t.binary,[...flags,'stop',t.process]);
  const start=()=>controller?controller.start():command(t.binary,[...flags,'start',t.process]);
  return {probe:async()=>{const info=await inspect();if(info.name!==t.process||!running(info)||!await up(t.port))throw new Error(`Process ${t.process} is not running and listening on port ${t.port}`);return {identity:identity(info)};},apply:async(_action,before)=>{if(!same(await inspect(),before))throw new Error('Process identity changed');const processes=await list();if(!Array.isArray(processes)||!processes.some(p=>p.name!==t.process&&running(p)))throw new Error('Interruption requires another running process to keep the process-compose controller alive');await stop();},isDown:async before=>{const info=await inspect();return same(info,before)&&!running(info)&&!await up(t.port);},restore:async before=>{const info=await inspect();if(!same(info,before))throw new Error('Process identity changed');if(running(info))return;await start();},verify:async before=>{const info=await inspect();return same(info,before)&&running(info)&&await up(t.port);}};
 }
 const listener=async()=>{let output;try{output=await command('lsof',['-nP',`-iTCP:${t.port}`,'-sTCP:LISTEN','-t']);}catch(e){if(e.code===1)return null;throw e;}const pids=[...new Set(output.trim().split(/\s+/).filter(Boolean))];if(!pids.length)return null;if(pids.length!==1)throw new Error('Expected one vault listener');const pid=Number(pids[0]);if(!Number.isSafeInteger(pid)||pid<=1)throw new Error('Invalid listener PID');const files=await command('lsof',['-a','-p',String(pid),'-d','cwd,txt','-Fn']);const names=files.split('\n').filter(line=>line.startsWith('n')).map(line=>line.slice(1));const repo=await realpath(t.repository);if(!names.includes(repo)||!names.some(name=>name.startsWith(repo+'/target/')&&path.basename(name)==='locker'))throw new Error('Port listener does not belong to the configured card-vault repository');const started=(await command('ps',['-p',String(pid),'-o','lstart='])).trim();if(!started)throw new Error('Vault process disappeared');return {pid,started,repository:repo};};
 const launch=hooks.launch||(async()=>{const marker=path.join(dataRoot,`fault-${t.id}-launcher.json`);const prior=await readJson(marker,null);if(prior){let alive=false;try{process.kill(prior.pid,0);alive=true;}catch(e){if(e.code!=='ESRCH')throw e;}if(alive)return;}const spec=await vaultLaunchSpec(t);const log=await open(path.join(dataRoot,`fault-${t.id}.log`),'a',0o600);try{const child=spawn(spec.binary,spec.args,{cwd:t.repository,detached:true,stdio:['ignore',log.fd,log.fd]});await new Promise((resolve,reject)=>{child.once('spawn',resolve);child.once('error',reject);});const info={pid:child.pid,at:Date.now()};const saved=atomicJson(marker,info);child.once('exit',(code,signal)=>{saved.then(()=>atomicJson(marker,{...info,exitCode:code,signal})).catch(()=>{});});await saved;child.unref();}finally{await log.close();}});
 return {probe:async()=>{const info=await listener();if(!info)throw new Error('Card vault is not listening');if(!hooks.launch){await vaultLaunchSpec(t);}return info;},apply:async(action,before)=>{const now=await listener();if(!now||now.pid!==before.pid||now.started!==before.started)throw new Error('Vault process identity changed');(hooks.kill||process.kill)(before.pid,'SIGTERM');const deadline=Date.now()+30000;while(await up(t.port)){if(Date.now()>deadline)throw new Error('Vault did not stop after SIGTERM');await pause(250);}},isDown:async()=>!await up(t.port),restore:async()=>{if(await up(t.port)){await listener();return;}await launch();},verify:async()=>{if(!await up(t.port)){if(!hooks.launch){const info=await readJson(path.join(dataRoot,`fault-${t.id}-launcher.json`),null);if(info&&('exitCode' in info))throw new Error(`Vault launcher exited ${info.exitCode ?? info.signal}; see fault-${t.id}.log`);}return false;}return Boolean(await listener());}};
}
