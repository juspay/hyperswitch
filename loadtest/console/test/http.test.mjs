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
  config.load={...config.load,total_rps:10,duration_seconds:4};
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
  async function completed(id){for(let i=0;i<150;i++){const job=await (await request(`/api/jobs/${id}`)).json();if(!['starting','running','cancelling'].includes(job.status))return job;await wait(100);}throw Error('Job timed out');}
  try {
    launch();await ready();
    assert.equal((await request('/api/settings','GET',null,false)).status,401);
    assert.equal((await request('/grafana/d-solo/k6/test','GET',null,false)).status,401);
    assert.equal((await request('/')).status,303);
    const response=await fetch(origin+'/login',{method:'POST',redirect:'manual',headers:{origin,accept:'text/html'},body:new URLSearchParams({username:'admin',password:'console test password'})});
    assert.equal(response.status,303);cookie=response.headers.get('set-cookie').split(';')[0];assert.match(response.headers.get('set-cookie'),/HttpOnly/);
    const page=await request('/');assert.equal(page.status,200);assert.match(await page.text(),/100% allocated/);
    const settings=await (await request('/api/settings')).json();assert.equal(settings.adminApiKey,'');assert.doesNotMatch(JSON.stringify(settings),/mock-private-admin|mock-sp-token|secret-grafana-test/);
    assert.equal((await request('/api/settings','PUT',settings,true,{origin:'https://other.example'})).status,403);
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
    assert.equal((await completed(fixtures.id)).status,'complete');
    const job=await (await request('/api/runs','POST',{mode:'single'})).json();
    const firstEvents=await request(`/api/jobs/${job.id}/events`),reader=firstEvents.body.getReader();
    assert.match(new TextDecoder().decode((await reader.read()).value),/event: progress/);await reader.cancel();
    let progress;
    for(let n=0;n<20;n++){await wait(250);progress=await (await request(`/api/jobs/${job.id}`)).json();if(progress.progress.elapsed_seconds>0)break;}
    assert.ok(progress.progress.elapsed_seconds>0,JSON.stringify(progress));
    settings.totalRps=11;settings.durationSeconds=12;assert.equal((await request('/api/settings','PUT',settings)).status,200);
    const immutable=await (await request(`/api/jobs/${job.id}`)).json();assert.equal(immutable.profile.totalRps,10);assert.equal(immutable.profile.durationSeconds,4);
    assert.match(immutable.runId,/-single-10tps-/);
    const snapshot=JSON.parse(await readFile(path.join(temporary,'jobs',job.id,'config.json'),'utf8'));
    assert.equal(snapshot.mode,'single');assert.equal(snapshot.load.total_rps,10);
    settings.totalRps=10;settings.durationSeconds=4;assert.equal((await request('/api/settings','PUT',settings)).status,200);
    const end=await completed(job.id);assert.equal(end.status,'complete',end.output);
    const run=await (await request(`/api/runs/${encodeURIComponent(end.runId)}`)).json();
    assert.ok(run.successfulTps>=9);assert.equal(run.gaps,0);assert.ok(run.complete);
    assert.ok(run.telemetry.length>=2);assert.ok(run.telemetry.at(-1).recorded>0);
    assert.ok(run.telemetry.every((point,index)=>!index || point.t>run.telemetry[index-1].t));
    const history=await (await request(`/api/runs/${encodeURIComponent(end.runId)}/requests?limit=2`)).json();assert.equal(history.rows.length,2);assert.ok(history.next);
    // Restart preserves sessions and completed jobs.
    server.kill('SIGTERM');await once(server,'exit');launch();await ready();
    assert.equal((await request('/api/settings')).status,200);assert.equal((await (await request(`/api/jobs/${job.id}`)).json()).status,'complete');
    settings.durationSeconds=20;assert.equal((await request('/api/settings','PUT',settings)).status,200);
    const cancelled=await (await request('/api/runs','POST',{mode:'single'})).json();await wait(1300);
    assert.equal((await request(`/api/jobs/${cancelled.id}/cancel`,'POST')).status,200);
    assert.equal((await completed(cancelled.id)).status,'cancelled');
    const replay=await request(`/api/jobs/${cancelled.id}/events`);assert.match(await replay.text(),/cancelled/);
    const shutdown=await (await request('/api/runs','POST',{mode:'single'})).json();await wait(1300);
    server.kill('SIGTERM');await once(server,'exit');launch();await ready();
    assert.equal((await (await request(`/api/jobs/${shutdown.id}`)).json()).status,'cancelled');
    const interrupted=await (await request('/api/runs','POST',{mode:'single'})).json();await wait(1300);
    server.kill('SIGKILL');await once(server,'exit');launch();await ready();
    assert.equal((await (await request(`/api/jobs/${interrupted.id}`)).json()).status,'interrupted');
    assert.equal((await (await request(`/api/runs/${encodeURIComponent(interrupted.runId)}`)).json()).complete,false);
    assert.equal((await request('/logout','POST')).status,303);assert.equal((await request('/api/settings')).status,401);
  } finally {server?.kill('SIGTERM');target.server.close();grafana.close();}
});
