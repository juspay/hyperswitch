import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp,readFile,writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { passwordHash } from '../server/auth.mjs';
import { mockServer } from '../../k6/scenario-mix/test/mock-server.mjs';

const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
test('SSR, authentication, Grafana proxy, jobs, SSE, cancellation and restart',async()=>{
  const temporary=await mkdtemp(path.join(os.tmpdir(),'loadtest-console-http-'));
  const target=await mockServer({listPolls:0,baselinePolls:0});
  let lastHeaders;
  let lastGrafanaPath;
  const grafana=createServer((req,res)=>{
    lastHeaders=req.headers;lastGrafanaPath=req.url;
    if(req.url==='/api/health'){res.setHeader('content-type','application/json');res.end('{"database":"ok","version":"12"}');return;}
    res.setHeader('content-type','text/html');res.end('<base href="/"><script>window.grafanaBootData={"appSubUrl":"","appUrl":"http://upstream/"}</script><script src="public/build/runtime.js"></script>');
  });
  grafana.listen(0,'127.0.0.1');await once(grafana,'listening');
  const port=39000+Math.floor(Math.random()*1000),origin=`http://127.0.0.1:${port}`;
  const config=JSON.parse(await readFile('../loadtest.config.example.json','utf8'));
  config.output=path.join(temporary,'artifacts');config.services={router:target.url,modular_pm:`${target.url}/v2`};
  config.fixtures.router=target.url;config.fixtures.admin_api_key='mock-private-admin';
  config.fixtures.superposition={endpoint:target.url,token:'mock-sp-token',organization_id:'org',workspace_id:'dev',refresh_interval_seconds:0,defaults:{flag:true}};
  config.load={...config.load,total_rps:10,duration_seconds:6};
  const seed=path.join(temporary,'seed.json');await writeFile(seed,JSON.stringify(config));
  const env={...process.env,HOST:'127.0.0.1',PORT:String(port),ORIGIN:origin,LOADTEST_DATA_DIR:temporary,
    LOADTEST_CONFIG_FILE:seed,ADMIN_PASSWORD_HASH:await passwordHash('console test password'),ADMIN_USERNAME:'admin',
    GRAFANA_URL:`http://127.0.0.1:${grafana.address().port}`,GRAFANA_TOKEN:'secret-grafana-test',NODE_ENV:'production'};
  let server,output='',cookie;
  function launch(){server=spawn(process.execPath,['start.mjs'],{cwd:path.resolve('.'),env,stdio:['ignore','pipe','pipe']});server.stdout.on('data',d=>output+=d);server.stderr.on('data',d=>output+=d);}
  async function ready(){for(let i=0;i<100;i++){try{if((await fetch(origin+'/health')).ok)return;}catch{}await wait(100);}throw Error(output);}
  async function request(url,method='GET',body=null,useCookie=true,customHeaders={}){
    return fetch(origin+url,{method,redirect:'manual',headers:{...(useCookie&&cookie?{cookie}:{}),origin,...(body?{'content-type':'application/json'}:{}),...customHeaders},...(body?{body:JSON.stringify(body)}:{})});
  }
  async function completed(id){for(let i=0;i<400;i++){const job=await (await request(`/api/jobs/${id}`)).json();if(!['starting','running','pausing','paused','cancelling'].includes(job.status))return job;await wait(100);}throw Error('Job timed out');}
  try {
    launch();await ready();
    assert.equal((await request('/api/settings','GET',null,false)).status,401);
    assert.equal((await request('/api/faults/targets','GET',null,false)).status,401);
    assert.equal((await request('/grafana/d-solo/k6/test','GET',null,false)).status,401);
    assert.equal((await request('/')).status,303);
    const response=await fetch(origin+'/login',{method:'POST',redirect:'manual',headers:{origin,accept:'text/html'},body:new URLSearchParams({username:'admin',password:'console test password'})});
    assert.equal(response.status,303);cookie=response.headers.get('set-cookie').split(';')[0];assert.match(response.headers.get('set-cookie'),/HttpOnly/);
    assert.equal((await request('/')).status,303);
    const page=await request('/scenarios');assert.equal(page.status,200);assert.match(await page.text(),/Saved scenarios/);
    const initialProfile=(await (await request('/api/profiles')).json())[0];
    const editor=await request(`/scenarios?view=edit&profile=${initialProfile.id}`);assert.equal(editor.status,200);assert.match(await editor.text(),/100% allocated/);
    assert.match(await (await request(`/scenarios?view=edit&profile=${initialProfile.id}`)).text(),/Fault targets/);
    const settings=await (await request('/api/settings')).json();assert.equal(settings.adminApiKey,'');assert.doesNotMatch(JSON.stringify(settings),/mock-private-admin|mock-sp-token|secret-grafana-test/);
    assert.equal((await request('/api/settings','PUT',settings,true,{origin:'https://other.example'})).status,403);
    const localTargets=[{id:'vault',label:'Vault test',adapter:'local-vault',port:3001,repository:temporary}];
    assert.equal((await request('/api/faults/targets','PUT',localTargets)).status,200);
    assert.equal((await (await request('/api/faults/targets')).json())[0].id,'vault');
    assert.ok((await (await request('/api/faults')).json()).targets.some(t=>t.id==='vault'));
    assert.equal((await request('/api/faults/targets','PUT',[],true,{origin:'https://other.example'})).status,403);
    assert.equal((await request('/api/faults/targets','PUT',[])).status,200);
    assert.equal((await request('/api/faults/test','POST',{targetId:'demo',action:'outage',confirm:'outage demo'},false)).status,401);
    const testResponse=await request('/api/faults/test','POST',{targetId:'demo',action:'outage',durationSeconds:30,confirm:'outage demo'});
    assert.equal(testResponse.status,202);const outageTest=await testResponse.json();assert.equal(outageTest.command,'fault-test');
    assert.equal((await request('/api/fixtures','POST',{mode:'single'})).status,409);
    // Reloading the browser does not own the standalone outage or recovery.
    assert.equal((await request('/scenarios')).status,200);
    for(let n=0;n<50;n++){if((await (await request(`/api/jobs/${outageTest.id}`)).json()).events.some(e=>e.kind==='fault_started'))break;await wait(20);}
    const restoreResponse=await request('/api/faults/restore','POST',{});assert.equal(restoreResponse.status,200);
    for(let n=0;n<100;n++){if(!(await (await request('/api/faults')).json()).active)break;await wait(20);}
    const tested=await completed(outageTest.id);assert.equal(tested.status,'complete');assert.ok(tested.events.some(e=>e.kind==='fault_restored'));
    let profile=await (await request('/api/profiles')).json();profile=profile[0];
    async function saveLoad(){profile=await (await request(`/api/profiles/${profile.id}`,'PUT',{...profile,...settings,name:'HTTP test profile',merchantCount:1})).json();assert.ok(profile.id,JSON.stringify(profile));}
    async function startLoad(){return (await request('/api/runs','POST',{mode:'single',profileId:profile.id,version:profile.version})).json();}
    assert.equal((await request('/api/settings/database/test','POST',{host:'127.0.0.1',port:1,database:'test',user:'missing',password:''})).status,400);
    for(const route of ['/fixtures','/performance','/results'])assert.equal((await request(route)).status,200);
    const resultsPage=await (await request('/results')).text();assert.doesNotMatch(resultsPage,/Run \/ scenario|run-library/);assert.match(resultsPage,/No runs yet|run-select/);
    const panels=[{title:'One',url:'/grafana/d-solo/k6/test?panelId=1'},{title:'Two',url:'/grafana/d-solo/k6/test?panelId=2'}];
    assert.equal((await request('/api/grafana/panels','PUT',panels)).status,200);
    assert.equal((await (await request('/api/grafana/panels')).json()).length,2);
    assert.equal((await request(panels[0].url)).status,200);assert.equal(lastHeaders.authorization,'Bearer secret-grafana-test');assert.equal(lastHeaders.cookie,undefined);
    assert.equal(lastGrafanaPath,'/d-solo/k6/test?panelId=1');
    const embedded=await (await request(panels[1].url)).text();
    assert.match(embedded,/<base href="\/grafana\/">/);assert.match(embedded,/"appSubUrl":"\/grafana"/);
    assert.equal((await request('/grafana/public/build/runtime.js')).status,200);assert.equal(lastGrafanaPath,'/public/build/runtime.js');
    assert.equal((await request('/grafana/api/admin/users','POST',{})).status,403);
    assert.equal((await request('/grafana/api/ds/queryevil','POST',{})).status,403);
    const fixtures=await (await request('/api/fixtures','POST',{mode:'single'})).json();
    assert.equal((await request('/api/runs','POST',{mode:'single'})).status,409);
    const completedFixtures=await completed(fixtures.id);assert.equal(completedFixtures.status,'complete');assert.equal(completedFixtures.progress.setup.stage,'ready');assert.equal(completedFixtures.progress.setup.superposition.status,'complete');
    const job=await startLoad();
    const firstEvents=await request(`/api/jobs/${job.id}/events`),reader=firstEvents.body.getReader();
    assert.match(new TextDecoder().decode((await reader.read()).value),/event: progress/);await reader.cancel();
    let progress;
    for(let n=0;n<20;n++){await wait(250);progress=await (await request(`/api/jobs/${job.id}`)).json();if(progress.progress.elapsed_seconds>0)break;}
    assert.ok(progress.progress.elapsed_seconds>0,JSON.stringify(progress));
    assert.equal((await request('/performance')).status,200);
    assert.equal((await (await request(`/api/jobs/${job.id}`)).json()).status,'running');
    assert.equal((await request('/api/faults','POST',{jobId:job.id,targetId:'demo',action:'outage'})).status,400);
    assert.equal((await request('/api/faults','POST',{jobId:job.id,targetId:'demo',action:'outage',durationSeconds:1,confirm:'outage demo'})).status,202);
    for(let n=0;n<50;n++){if(!(await (await request('/api/faults')).json()).active)break;await wait(100);}
    const pause=await request(`/api/jobs/${job.id}/pause`,'POST');assert.equal(pause.status,200,await pause.clone().text());
    let paused;
    for(let n=0;n<50;n++){paused=await (await request(`/api/jobs/${job.id}`)).json();if(paused.loadState==='paused')break;await wait(100);}
    assert.equal(paused.loadState,'paused');assert.equal(paused.progress.control.in_flight,0);
    assert.equal((await request('/api/runs','POST',{mode:'single'})).status,409);
    await wait(1300);
    const stillPaused=await (await request(`/api/jobs/${job.id}`)).json();assert.equal(stillPaused.progress.control.active_seconds,paused.progress.control.active_seconds);
    assert.equal((await request(`/api/jobs/${job.id}/resume`,'POST')).status,200);
    settings.totalRps=11;settings.durationSeconds=12;assert.equal((await request('/api/settings','PUT',settings)).status,200);await saveLoad();
    const immutable=await (await request(`/api/jobs/${job.id}`)).json();assert.equal(immutable.profile.totalRps,10);assert.equal(immutable.profile.durationSeconds,6);
    assert.match(immutable.runId,/-single-10tps-/);
    const snapshot=JSON.parse(await readFile(path.join(temporary,'jobs',job.id,'config.json'),'utf8'));
    assert.equal(snapshot.mode,'single');assert.equal(snapshot.load.total_rps,10);
    settings.totalRps=10;settings.durationSeconds=6;assert.equal((await request('/api/settings','PUT',settings)).status,200);await saveLoad();
    const end=await completed(job.id);assert.equal(end.status,'complete',end.output);
    const run=await (await request(`/api/runs/${encodeURIComponent(end.runId)}`)).json();
    assert.equal(run.successfulTps,10);assert.equal(run.iterations,60);assert.equal(run.gaps,0);assert.ok(run.complete);assert.ok(run.confirmP99!=null);assert.equal(run.acceptance.resilience,'simulated');assert.ok(run.activity.some(e=>e.kind==='pause'));assert.ok(run.activity.some(e=>e.kind==='fault_restored'));
    assert.ok(run.telemetry.length>=2);assert.ok(run.telemetry.at(-1).recorded>0);
    assert.ok(run.telemetry.every((point,index)=>!index || point.t>run.telemetry[index-1].t));
    const history=await (await request(`/api/runs/${encodeURIComponent(end.runId)}/requests?limit=2`)).json();assert.equal(history.rows.length,2);assert.ok(history.next);
    settings.loadShape='staged';settings.startRps=0;settings.stagesJson=JSON.stringify([{target_rps:10,duration_seconds:4}]);
    assert.equal((await request('/api/settings','PUT',settings)).status,200);await saveLoad();
    const staged=await startLoad();
    assert.equal((await completed(staged.id)).status,'complete');
    const stagedRun=await (await request(`/api/runs/${encodeURIComponent(staged.runId)}`)).json();
    assert.equal(stagedRun.durationSeconds,4);assert.equal(stagedRun.offeredTps,10);assert.equal(stagedRun.expectedAverageTps,5);
    assert.ok(stagedRun.iterations>=18&&stagedRun.iterations<=21,JSON.stringify(stagedRun));assert.equal(stagedRun.gaps,0);
    settings.startRps=10;settings.stagesJson=JSON.stringify([{target_rps:10,duration_seconds:3,fault:{targetId:'demo',action:'outage'}},{target_rps:10,duration_seconds:3}]);
    await saveLoad();const scheduled=await startLoad();
    const scheduledEnd=await completed(scheduled.id);assert.equal(scheduledEnd.status,'complete',JSON.stringify(scheduledEnd));
    assert.equal(scheduledEnd.events.filter(e=>e.kind==='scheduled_fault').length,1);
    assert.equal(scheduledEnd.events.filter(e=>e.kind==='fault_started').length,1);
    assert.equal(scheduledEnd.events.filter(e=>e.kind==='fault_restored').length,1);
    assert.equal(scheduledEnd.profile.timeline.at(-1).end,6);
    const plannedRun=await (await request(`/api/runs/${encodeURIComponent(scheduled.runId)}`)).json();assert.equal(plannedRun.timeline[0].fault.action,'outage');assert.equal(plannedRun.timeline.at(-1).end,6);assert.doesNotMatch(JSON.stringify(plannedRun.timeline),/secret|repository|controllerPort/);
    const plannedPage=await (await request(`/performance?run=${encodeURIComponent(scheduled.runId)}`)).text();assert.match(plannedPage,/recovery begins \(planned\)/);assert.match(plannedPage,/over the full 6 second test/);
    const faultSnapshot=JSON.parse(await readFile(path.join(temporary,'jobs',scheduled.id,'config.json'),'utf8'));
    assert.equal(faultSnapshot.faultPlan[0].target.id,'demo');assert.equal(faultSnapshot.faultPlan[0].at,0);
    settings.loadShape='flat';assert.equal((await request('/api/settings','PUT',settings)).status,200);await saveLoad();
    target.state.errorCustomers=true;settings.durationSeconds=3;assert.equal((await request('/api/settings','PUT',settings)).status,200);await saveLoad();
    const failing=await startLoad();
    const failed=await completed(failing.id);assert.equal(failed.status,'failed');assert.equal(failed.exitCode,99);
    const failedRun=await (await request(`/api/runs/${encodeURIComponent(failing.runId)}`)).json();assert.ok(failedRun.complete);assert.equal(failedRun.acceptance.verdict,'fail');
    target.state.errorCustomers=false;settings.durationSeconds=6;assert.equal((await request('/api/settings','PUT',settings)).status,200);await saveLoad();
    // Restart preserves sessions and completed jobs.
    server.kill('SIGTERM');await once(server,'exit');launch();await ready();
    assert.equal((await request('/api/settings')).status,200);assert.equal((await (await request(`/api/jobs/${job.id}`)).json()).status,'complete');
    settings.durationSeconds=20;assert.equal((await request('/api/settings','PUT',settings)).status,200);await saveLoad();
    const cancelled=await startLoad();await wait(1300);
    assert.equal((await request(`/api/jobs/${cancelled.id}/cancel`,'POST')).status,200);
    assert.equal((await completed(cancelled.id)).status,'cancelled');
    const replay=await request(`/api/jobs/${cancelled.id}/events`);assert.match(await replay.text(),/cancelled/);
    const shutdown=await startLoad();await wait(1300);
    server.kill('SIGTERM');await once(server,'exit');launch();await ready();
    assert.ok(['running','complete'].includes((await (await request(`/api/jobs/${shutdown.id}`)).json()).status));
    assert.equal((await completed(shutdown.id)).status,'complete');
    const interrupted=await startLoad();await wait(1300);
    server.kill('SIGKILL');await once(server,'exit');launch();await ready();
    assert.ok(['running','complete'].includes((await (await request(`/api/jobs/${interrupted.id}`)).json()).status));
    await request(`/api/jobs/${interrupted.id}/cancel`,'POST');await completed(interrupted.id);
    // A worker crash is different from an SSR restart: recover and mark incomplete.
    const orphan=await startLoad();await wait(1300);
    const daemon=JSON.parse(await readFile(path.join(temporary,'worker-service.json'),'utf8'));
    process.kill(daemon.pid,'SIGKILL');
    const recovered=await (await request(`/api/jobs/${orphan.id}`)).json();assert.equal(recovered.status,'interrupted');
    assert.equal((await (await request(`/api/runs/${encodeURIComponent(orphan.runId)}`)).json()).complete,false);
    assert.equal((await request('/logout','POST')).status,303);assert.equal((await request('/api/settings')).status,401);
  } finally {server?.kill('SIGTERM');const daemon=JSON.parse(await readFile(path.join(temporary,'worker-service.json'),'utf8').catch(()=>'null'));if(daemon)try{process.kill(daemon.pid,'SIGTERM');}catch{}target.server.close();grafana.close();}
});
