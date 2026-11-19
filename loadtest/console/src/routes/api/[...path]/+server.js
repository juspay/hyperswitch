import {csvStream} from '../../../../server/csv-stream.mjs';
import { json } from '@sveltejs/kit';
import { publicSettings,saveSettings,saveConnections,localSettings,localPath,atomicJson,panelPath,redact,runDirectory,dataRoot,superpositionConnection } from '../../../../server/state.mjs';
import { queryWorker } from '../../../../server/workers.mjs';
import {workerCommand,jobEvents} from '../../../../server/supervisor.mjs';
import {listProfiles,getProfile,saveProfile,listFixtureSets,archiveProfile,deleteProfile} from '../../../../server/profiles.mjs';
import {testServiceEndpoint,testSuperpositionEndpoint} from '../../../../server/service-checks.mjs';
import {listCertificates,importCertificate,removeCertificate} from '../../../../server/certificates.mjs';
import {listWorkers,enrollWorker,heartbeatWorker,pendingWorkerCommands,acknowledgeWorkerCommand,removeWorker} from '../../../../server/worker-registry.mjs';
import {readWorkerBundle,storeWorkerArtifact} from '../../../../server/worker-artifacts.mjs';

async function dispatch(event) {
  const method=event.request.method,segments=(event.params.path || '').split('/');
  const input=async()=>{
    const text=await event.request.text();if(text.length>1024*1024)throw new Error('Request body exceeds 1 MiB');return text?JSON.parse(text):{};
  };
  try {
    if(segments[0]==='worker'&&segments[1]==='v1'){
      const bearer=(event.request.headers.get('authorization')||'').replace(/^Bearer\s+/i,'');
      if(segments[2]==='enroll'&&segments.length===3&&method==='POST')return json(enrollWorker(await input(),bearer),{status:201});
      const id=segments[3];
      if(segments[2]==='workers'&&segments[4]==='heartbeat'&&segments.length===5&&method==='POST')return json(heartbeatWorker(id,bearer,await input()));
      if(segments[2]==='workers'&&segments[4]==='commands'&&segments.length===5&&method==='GET')return json(pendingWorkerCommands(id,bearer,Number(event.url.searchParams.get('after')||0)));
      if(segments[2]==='workers'&&segments[4]==='commands'&&segments[6]==='ack'&&segments.length===7&&method==='POST')return json(acknowledgeWorkerCommand(id,bearer,segments[5],await input()));
      if(segments[2]==='workers'&&segments[4]==='bundles'&&segments.length===6&&method==='GET'){
        const {authenticateWorker}=await import('../../../../server/worker-registry.mjs');authenticateWorker(id,bearer);const bundle=await readWorkerBundle(segments[5],id);return new Response(bundle.body,{headers:{'content-type':'application/json','x-content-sha256':bundle.sha256}});
      }
      if(segments[2]==='workers'&&segments[4]==='runs'&&segments[6]==='artifacts'&&segments.length===8&&method==='PUT'){
        const {authenticateWorker}=await import('../../../../server/worker-registry.mjs');authenticateWorker(id,bearer);if(!event.request.body)throw new Error('Artifact body is required');
        return json(await storeWorkerArtifact({workerId:id,runId:segments[5],name:segments[7],body:event.request.body,contentLength:Number(event.request.headers.get('content-length')||0)}),{status:201});
      }
      throw new Error('Unknown worker protocol operation');
    }
    if(segments.join('/')==='workers/plan'&&method==='GET'){const {planWorkerShards}=await import('../../../../server/distributed.mjs');const {loadShape}=await import('../../../load-shape.mjs');const profile=getProfile(event.url.searchParams.get('profile')),settings=publicSettings(),peak=loadShape(profile.load).peak;return json(planWorkerShards(peak,Math.ceil(peak*settings.maxVusMultiplier),listWorkers().filter(w=>w.id!=='local')));}
    if(segments[0]==='workers'&&segments.length===1&&method==='GET')return json(listWorkers());
    if(segments[0]==='workers'&&segments.length===2&&method==='DELETE')return json(removeWorker(segments[1]));
    if(segments.join('/')==='settings/certificates') {
      if(method==='GET')return json(listCertificates(dataRoot));
      if(method==='POST'){
        const length=Number(event.request.headers.get('content-length')||0);if(length>1100000)throw new Error('Certificate upload exceeds 1 MiB');
        const form=await event.request.formData(),file=form.get('certificate');
        if(!file||typeof file.arrayBuffer!=='function'||file.size>1024*1024)throw new Error('Choose a certificate up to 1 MiB');
        const entry=await importCertificate(dataRoot,{name:form.get('name')||file.name,pem:Buffer.from(await file.arrayBuffer()).toString('utf8')});
        return json(entry,{status:201});
      }
    }
    if(segments[0]==='settings'&&segments[1]==='certificates'&&segments.length===3&&method==='DELETE'){
      const settings=localSettings();if(settings.database.caCertificateId===segments[2])throw new Error('Remove this certificate from the database settings before deleting it');
      await removeCertificate(dataRoot,segments[2]);return json({deleted:true});
    }
    if(segments[0]==='settings' && segments.length===1) {
      if(method==='GET')return json(publicSettings());
      if(method==='PUT')return json(await saveConnections(await input()));
    }
    if(segments[0]==='profiles') {
      if(segments.length===1&&method==='GET')return json(listProfiles({includeArchived:event.url.searchParams.get('archived')==='true'}));
      if(segments.length===3&&segments[2]==='archive'&&method==='PUT')return json(archiveProfile(segments[1],(await input()).archived===true));
      if(segments.length===1&&method==='POST')return json(saveProfile(await input()),{status:201});
      if(segments.length===2&&method==='GET')return json(getProfile(segments[1]));
      if(segments.length===2&&method==='PUT')return json(saveProfile(await input(),segments[1]));
      if(segments.length===2&&method==='DELETE')return json(deleteProfile(segments[1]));
    }
    if(segments.join('/')==='scenarios/save'&&method==='POST')return json(await workerCommand('save-scenario',await input()));
    if(segments.join('/')==='settings/database/test'&&method==='POST') {
      const draft=await input(),saved=localSettings().database;
      const database={...saved,...draft,password:draft.password || saved.password};
      return json(await queryWorker('database-test',{database}));
    }
    if(segments.join('/')==='settings/services/test'&&method==='POST') {
      const body=await input();
      return json(body.kind==='superposition'
        ? await testSuperpositionEndpoint(superpositionConnection(body.superposition))
        : await testServiceEndpoint(body.kind,body.endpoint));
    }
    if(segments[0]==='fixtures'&&segments.length===1&&method==='GET')return json(listFixtureSets());
    if(segments[0]==='fixtures' && segments.length===1 && method==='POST')return json(await workerCommand('start',{...await input(),kind:'fixtures'}),{status:202});
    if(segments[0]==='smoke' && segments.length===1 && method==='POST')return json(await workerCommand('start',{...await input(),kind:'smoke'}),{status:202});
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
      const args={...Object.fromEntries(event.url.searchParams),id:segments[1]};
      if(segments.length===3&&method==='GET'&&['payment-bars','payment-page','payment-detail','next-discrepancy'].includes(segments[2]))return json(await queryWorker(segments[2],args));
      if(segments.length===3&&method==='GET'&&segments[2]==='export'){
        const payments=event.url.searchParams.get('kind')==='payments',columns=payments?['elapsed_ms','merchant_id','payment_id','recorded_status','database_status','outcome','failure_category','event_id','operation','api_status_code','message']:['event_id','observed_at','operation','merchant_id','payment_id','api_status_code','status','latency_ms','message'];
        // Validate run ownership/path before beginning the response stream.
        runDirectory(segments[1]);
        return new Response(csvStream(payments?'payment-page':'requests',args,columns),{headers:{'content-type':'text/csv; charset=utf-8','content-disposition':`attachment; filename="${payments?'payments':'responses'}.csv"`}});
      }
      if(segments.length===1 && method==='POST')return json(await workerCommand('start',{...await input(),kind:'load'}),{status:202});
      if(segments.length===1 && method==='GET')return json(await queryWorker('list'));
      if(segments.length===2 && method==='GET')return json(await queryWorker('run',{id:segments[1]}));
      if(segments.length===3 && segments[2]==='requests' && method==='GET') {
        const after=Number(event.url.searchParams.get('after') || 0),limit=Number(event.url.searchParams.get('limit') || 100);
        if(!Number.isSafeInteger(after)||after<0||!Number.isInteger(limit)||limit<1||limit>200)throw new Error('Invalid pagination');
        return json(await queryWorker('requests',{...args,after,limit}));
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
export const GET=dispatch,POST=dispatch,PUT=dispatch,DELETE=dispatch;
