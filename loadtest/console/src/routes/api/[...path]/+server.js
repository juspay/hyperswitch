import { json } from '@sveltejs/kit';
import { publicSettings,saveSettings,saveConnections,localSettings,localPath,atomicJson,panelPath,redact,runDirectory } from '../../../../server/state.mjs';
import { queryWorker } from '../../../../server/workers.mjs';
import {workerCommand,jobEvents} from '../../../../server/supervisor.mjs';
import {listProfiles,getProfile,saveProfile,listFixtureSets} from '../../../../server/profiles.mjs';

async function dispatch(event) {
  const method=event.request.method,segments=(event.params.path || '').split('/');
  const input=async()=>{
    const text=await event.request.text();if(text.length>1024*1024)throw new Error('Request body exceeds 1 MiB');return text?JSON.parse(text):{};
  };
  try {
    if(segments[0]==='settings' && segments.length===1) {
      if(method==='GET')return json(publicSettings());
      if(method==='PUT')return json(await saveConnections(await input()));
    }
    if(segments[0]==='profiles') {
      if(segments.length===1&&method==='GET')return json(listProfiles());
      if(segments.length===1&&method==='POST')return json(saveProfile(await input()),{status:201});
      if(segments.length===2&&method==='GET')return json(getProfile(segments[1]));
      if(segments.length===2&&method==='PUT')return json(saveProfile(await input(),segments[1]));
    }
    if(segments.join('/')==='settings/database/test'&&method==='POST') {
      const draft=await input(),saved=localSettings().database;
      const database={...saved,...draft,password:draft.password || saved.password};
      return json(await queryWorker('database-test',{database}));
    }
    if(segments[0]==='fixtures'&&segments.length===1&&method==='GET')return json(listFixtureSets());
    if(segments[0]==='fixtures' && segments.length===1 && method==='POST')return json(await workerCommand('start',{...await input(),kind:'fixtures'}),{status:202});
    if(segments[0]==='faults') {
      if(segments[1]==='targets'&&segments.length===2&&method==='GET')return json(await workerCommand('fault-targets'));
      if(segments[1]==='targets'&&segments.length===2&&method==='PUT')return json(await workerCommand('save-fault-targets',{targets:await input()}));
      if(segments[1]==='test'&&segments.length===2&&method==='POST')return json(await workerCommand('test-fault',await input()),{status:202});
      if(segments[1]==='check'&&segments.length===2&&method==='POST')return json(await workerCommand('check-fault-target',await input()));
      if(segments.length===1&&method==='GET')return json(await workerCommand('faults'));
      if(segments[1]==='restore'&&segments.length===2&&method==='POST')return json(await workerCommand('restore'));
      if(segments.length===1&&method==='POST'){const body=await input();return json(await workerCommand('fault',body),{status:202});}
    }
    if(segments[0]==='jobs') {
      if(segments.length===1 && method==='GET')return json(await workerCommand('active'));
      if(segments.length===2 && method==='GET')return json(await workerCommand('job',{id:segments[1]}));
      if(segments[2]==='events' && segments.length===3 && method==='GET')return jobEvents(segments[1],event.request);
      if(segments[2]==='cancel' && segments.length===3 && method==='POST')return json(await workerCommand('cancel',{id:segments[1]}));
      if(['pause','resume'].includes(segments[2]) && segments.length===3 && method==='POST')return json(await workerCommand('control',{id:segments[1],action:segments[2]}));
    }
    if(segments[0]==='runs') {
      if(segments.length===1 && method==='POST')return json(await workerCommand('start',{...await input(),kind:'load'}),{status:202});
      if(segments.length===1 && method==='GET')return json(await queryWorker('list'));
      if(segments.length===2 && method==='GET')return json(await queryWorker('run',{id:segments[1]}));
      if(segments.length===3 && segments[2]==='requests' && method==='GET') {
        const after=Number(event.url.searchParams.get('after') || 0),limit=Number(event.url.searchParams.get('limit') || 100);
        if(!Number.isSafeInteger(after)||after<0||!Number.isInteger(limit)||limit<1||limit>200)throw new Error('Invalid pagination');
        return json(await queryWorker('requests',{id:segments[1],after,limit}));
      }
      if(segments.length===3 && segments[2]==='reconcile' && method==='GET')return json(await workerCommand('latest-reconciliation',{path:runDirectory(segments[1]).path}));
      if(segments.length===3 && segments[2]==='reconcile' && method==='POST') {
        const body=await input();if(body.allowIncomplete!==undefined && typeof body.allowIncomplete!=='boolean')throw new Error('allowIncomplete must be a boolean');
        return json(await workerCommand('reconcile',{id:segments[1],allowIncomplete:body.allowIncomplete===true}),{status:202});
      }
    }
    if(segments.join('/')==='grafana/panels') {
      const local=localSettings();
      if(method==='GET')return json(local.grafana.panels || []);
      if(method==='PUT') {
        const panels=await input();if(!Array.isArray(panels)||panels.length>20)throw new Error('Choose at most 20 panels');
        local.grafana.panels=panels.map(p=>({title:String(p.title).slice(0,100),url:panelPath(p.url,local.grafana.baseUrl)}));
        await atomicJson(localPath,local);return json(local.grafana.panels);
      }
    }
    return json({error:'Not found'},{status:404});
  }catch(e){return json({error:redact(e.message)},{status:e.status || 400});}
}
export const GET=dispatch,POST=dispatch,PUT=dispatch;
