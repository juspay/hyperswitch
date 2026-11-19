<script>
  import {faultNotice} from './fault-notice.mjs';
  import {recordingFinished} from './run-lifecycle.mjs';
import Icon from './Icon.svelte';
  import { onMount, tick } from 'svelte';
  import { api } from './api.js';
  import StatCard from './StatCard.svelte';
  import {loadShape,loadTimeline} from './load-shape.mjs';
  import LiveChart from './LiveChart.svelte';
  import {compactSamples} from './chart-timeline.mjs';
  import FollowingLog from './FollowingLog.svelte';
  import {page} from '$app/stores';
  import {setQuery} from './navigation.mjs';
  import {watchJobStream} from './job-stream.mjs';
  import ConnectionStatus from './ConnectionStatus.svelte';
  import PaymentChart from './PaymentChart.svelte';
  import ResponseBrowser from './ResponseBrowser.svelte';
  import ReconciliationSummary from './ReconciliationSummary.svelte';
  import RunSummary from './RunSummary.svelte';
  export let data;

  const flowCatalog = [
    ['cit_off_session', 'CIT off-session'], ['sdk_checkout', 'SDK checkout'],
    ['guest', 'Guest payment'], ['cit_on_session', 'CIT on-session'],
    ['ptv_on_session', 'Pay to vault · on-session'], ['ptv_off_session', 'Pay to vault · off-session'],
    ['cit_metadata_changed', 'Card metadata update'], ['mit', 'Merchant-initiated payment'], ['saved_card_checkout', 'Saved-card checkout']
  ];
  import {goto} from '$app/navigation';
  export let tab='performance';

  let pending = '';
  let stopRequestedJobId = '';
  $: stopping = activeJob && (job?.status === 'cancelling' || stopRequestedJobId === job?.id);
  let notice = '';
  let error = '';
  let settings = structuredClone(data.settings);
  let workers=[],execution='local',workerPlan=null,planError='',planKey='',connection='connecting',lastUpdate=0,smokeDialog;
  $: if(execution==='distributed'&&executionProfile){const key=executionProfile.id+':'+executionProfile.version;if(key!==planKey){planKey=key;previewWorkers();}}
  async function previewWorkers(){try{workerPlan=await api.workerPlan(executionProfile.id);planError='';}catch(e){workerPlan=null;planError=e.message;}}
  let smokeRps=settings.smokeTest?.rps ?? 1,smokeDuration=settings.smokeTest?.durationSeconds ?? 10;
  $: shapePreview=previewShape(executionProfile || settings);
  function previewShape(s){try{return s.loadShape==='staged'?loadShape({start_rps:Number(s.startRps),stages:JSON.parse(s.stagesJson)}):{peak:s.totalRps,duration:s.durationSeconds};}catch{return {peak:'—',duration:'—'};}}
  $: activeJob = ['starting','running','pausing','paused','cancelling'].includes(job?.status);
  $: smokeActive=job?.command==='smoke' && activeJob;
  let runs = data.runs;
  let selectedRun = data.run || null;
  let runTimeline = data.run?.timeline || [];
  let job = data.job;
  let events;
  let finishedRecordingJobId='';
  let faultDialog;
  let faults={targets:[],active:null,blocked:false};
  let faultDraft={targetId:'demo',action:'outage',durationSeconds:10,confirm:''};
  let flowFilter=$page.url.searchParams.get('flow')||'all';
  let chartMetric=$page.url.searchParams.get('metric')||'tps';
  let urlReady=false;
  $: if(urlReady)setQuery({flow:flowFilter==='all'?null:flowFilter,metric:chartMetric==='tps'?null:chartMetric,time:null});
  $: if(['errors','backlog'].includes(chartMetric))flowFilter='all';
  $: viewJob=job?.command==='load' && (!selectedRun || job.runId===selectedRun.id) ? job : null;
  $: loadState=selectedRun?.live?viewJob?.loadState||selectedRun.loadState||'running':selectedRun?.loadState||viewJob?.loadState||'idle';
  $: live=selectedRun?.live && activeJob && (job?.runId===selectedRun.id || (job?.command==='smoke'&&selectedRun.id===`smoke:${job.id}`));
  $: chartPoints=selectedRun?.telemetry || [];
  $: faultTarget=faults.targets.find(t=>t.id===faultDraft.targetId);
  $: activity=(live?job?.events:selectedRun?.activity) || viewJob?.events || [];

  let reconciliationJob=data.reconciliationJob || null,reconciling=['starting','running','cancelling'].includes(data.reconciliationJob?.status),reconciliationEvents;
  let reconciliation = selectedRun?.reconciliation || null;
  let allowIncomplete = false,responsesOpen=false;


  onMount(() => {
    let disposed=false;
    tick().then(()=>{if(disposed)return;setQuery({run:selectedRun?.id||null,profile:selectedRun?.profile?.id||selectedProfileId,fixtures:selectedRun?.fixtureSetId||selectedFixtureId});urlReady=true;});
    if(['load','smoke'].includes(job?.command)) watchJob();
    if(reconciling)watchReconciliation(selectedRun.id);
    const connections=async()=>{try{[settings,workers]=await Promise.all([api.settings(),api.workers()]);faults=await api.faults();if(!faults.targets.some(t=>t.id===faultDraft.targetId)){faultDraft.targetId=faults.targets[0]?.id||'';faultDraft.action=faults.targets[0]?.actions[0]||'outage';}fixtureSets=await api.fixtures();error='';}catch(e){error=e.message;}};window.addEventListener('console-connections-saved',connections);connections();
    const polling=setInterval(async()=>{try{faults=await api.faults();workers=await api.workers();const r=await fetch('/api/jobs');if(r.ok){const active=await r.json();generatorBusy=active!=null && active.command!=='reconcile';if(['load','smoke'].includes(active?.command)&&active.id!==job?.id){job=active;watchJob();}}}catch{}},3000);
    api.faults().then(value=>{faults=value;faultDraft.targetId=value.targets[0]?.id || '';faultDraft.action=value.targets[0]?.actions[0] || 'outage';}).catch(e=>error=e.message);
    return () => {disposed=true;urlReady=false;events?.close();reconciliationEvents?.close();clearInterval(polling);window.removeEventListener('console-connections-saved',connections);};
  });

  async function refreshRuns() { runs = await api.runs(); }

  async function selectRun(run) {
    error = ''; reconciliation = null;
 flowFilter='all';
    try { selectedRun = await api.run(run.id); runTimeline=selectedRun.timeline || []; reconciliation = selectedRun.reconciliation || null;if(tab==='results'){reconciliationEvents?.close();reconciliationJob=await api.reconciliationJob(run.id);reconciling=['starting','running','cancelling'].includes(reconciliationJob?.status);if(reconciling)watchReconciliation(run.id);}setQuery({run:run.id,profile:selectedRun.profile?.id||selectedProfileId,fixtures:selectedRun.fixtureSetId||selectedFixtureId});}
    catch (e) { error = e.message; }
  }

  let generatorBusy=data.busy || false;
  let profiles=data.profiles || [],fixtureSets=data.fixtures || [];
  let selectedProfileId=data.profileId || profiles[0]?.id || '';
  let selectedFixtureId=data.fixtureSetId || '';
  $: if(profiles.length&&!profiles.some(p=>p.id===selectedProfileId))selectedProfileId=profiles[0].id;
  $: executionProfile=profiles.find(p=>p.id===selectedProfileId);
  const readyFixturesFor=profile=>fixtureSets.filter(f=>f.status==='ready'&&f.compatible&&f.mode===profile?.mode&&f.merchant_count===profile?.merchantCount);
  $: compatibleFixtureSets=readyFixturesFor(executionProfile);
  $: if(!selectedRun&&!compatibleFixtureSets.some(f=>f.id===selectedFixtureId))selectedFixtureId=compatibleFixtureSets[0]?.id||'';
  function faultAction(stage){return faults.targets.find(target=>target.id===stage.fault?.targetId)?.actions?.[0] || stage.fault?.action;}
  function canonicalLoad(profile){const load=profile?.load || {};return {...load,stages:(load.stages || []).map(stage=>stage.fault?{...stage,fault:{...stage.fault,action:faultAction(stage)}}:stage)};}
  function chooseProfile(){const first=readyFixturesFor(profiles.find(p=>p.id===selectedProfileId))[0];selectedFixtureId=first?.id||'';selectedRun=null;goto(`/performance?profile=${encodeURIComponent(selectedProfileId)}${selectedFixtureId?`&fixtures=${encodeURIComponent(selectedFixtureId)}`:''}`,{replaceState:true,noScroll:true});}
  function newRun(){selectedRun=null;setQuery({run:null,profile:selectedProfileId});}
  function runAgain(){if(selectedRun?.profile?.id)selectedProfileId=selectedRun.profile.id;if(selectedRun?.fixtureSetId)selectedFixtureId=selectedRun.fixtureSetId;newRun();}
  $: startBlocker=generatorBusy||activeJob?'A job is already running.':!executionProfile?'Choose a saved scenario.':!selectedFixtureId?'Choose ready fixtures for this scenario.':execution==='distributed'&&planError?planError:'';
  async function start(command) {
    if(pending||startBlocker)return;
    if(!executionProfile||!selectedFixtureId){error='Select a saved scenario and ready fixtures';return;}
    pending=command;error='';smokeDialog?.close();
    runTimeline=command==='smoke'?[{index:0,start:0,end:Number(smokeDuration),from:Number(smokeRps),target:Number(smokeRps),kind:'hold',fault:null}]:loadTimeline(canonicalLoad(executionProfile),(executionProfile.load.stages || []).map((s,index)=>({index,target:{label:faults.targets.find(t=>t.id===s.fault?.targetId)?.label}})));
    try{
      const selection={mode:executionProfile.mode,profileId:executionProfile.id,version:executionProfile.version,fixtureSetId:selectedFixtureId,execution};
      job=await api.start(command,command==='smoke'?{...selection,rps:Number(smokeRps),durationSeconds:Number(smokeDuration)}:selection);
      notice='';selectedRun=null;flowFilter='all';generatorBusy=true;watchJob();
      if(command==='load')goto(`/performance?run=${encodeURIComponent(job.runId)}&profile=${encodeURIComponent(selectedProfileId)}`,{replaceState:true,noScroll:true});
    }
    catch(e){error=e.message;}finally{pending='';}
  }

  function watchJob() {
    events?.close();
    events = watchJobStream(job.id, async (value) => {
      job = value;
      if(job.faults){faults=job.faults;window.dispatchEvent(new Event('console-fault-state'));}
      const profile=job.profile || settings;
      const visualId=job.command==='smoke'?`smoke:${job.id}`:job.runId;
      if(visualId && (!selectedRun || selectedRun.id === visualId)) {
        const p=job.progress;
        let telemetry=selectedRun?.telemetry || [];
        if(p.trend && (!telemetry.length || p.trend.t>telemetry.at(-1).t)) {
          const trend={...p.trend,serverP99:p.trend.serverP99 ?? p.server_p99_ms ?? null,flows:Object.fromEntries(Object.entries(p.trend.flows||{}).map(([name,f])=>[name,{...f,serverP99:f.serverP99 ?? p.flows?.[name]?.server_p99_ms ?? null}]))};
          telemetry=compactSamples([...telemetry,trend]);
        }
        if(p.elapsed_seconds!=null&&!recordingFinished(job)) selectedRun={...(selectedRun || {}),id:visualId,kind:job.command,mode:job.mode,startedAt:job.startedAt,profile:job.profile,
          telemetry,timeline:job.profile?.timeline || selectedRun?.timeline || runTimeline,
          activity:job.events,activeSeconds:p.control?.active_seconds ?? p.elapsed_seconds,wallSeconds:p.control?.wall_seconds ?? p.elapsed_seconds,loadState:job.loadState,serverP99:p.server_p99_ms ?? null,confirmP99:p.p99_ms ?? null,
          live:true,complete:false,recordingState:job.command==='smoke'?'disabled':'running',durationSeconds:profile.durationSeconds,
          elapsedSeconds:Number((p.control?.active_seconds ?? p.elapsed_seconds).toFixed(1)),iterations:p.iterations,successfulTps:Number(p.successful_tps.toFixed(2)),
          displayErrorRate:p.display_error_rate ?? (p.http_attempts?100*p.http_errors/p.http_attempts:0),
          confirmErrorRate:p.confirm_http_attempts==null?null:p.confirm_http_attempts?Number((p.confirm_http_errors/p.confirm_http_attempts*100).toFixed(2)):0,
          offeredTps:profile.totalRps,errorRate:p.http_attempts?Number((p.http_errors/p.http_attempts*100).toFixed(2)):0,
          dropped:p.dropped,enqueued:p.recorder.enqueued,persisted:p.recorder.persisted,gaps:p.recorder.enqueued-p.recorder.persisted,
          flows:profile.scenarios.map(f=>({name:f.name,label:flowCatalog.find(c=>c[0]===f.scenario)?.[1] || f.name,
            offeredTps:profile.totalRps*f.weight/100,successfulTps:Number((p.flows[f.name]?.tps || 0).toFixed(2)),
            serverP99:p.flows[f.name]?.server_p99_ms ?? null,confirmP99:p.flows[f.name]?.p99_ms ?? null,confirmP90:p.flows[f.name]?.p90_ms ?? null,errors:p.flows[f.name]?.failures || 0}))};
      }
      if(recordingFinished(job)&&finishedRecordingJobId!==job.id){
        finishedRecordingJobId=job.id;
        try{
          await refreshRuns();
          const recorded=runs.find(r=>r.id===job.runId);
          if(!recorded?.endedAt||recorded.recordingState==='running')finishedRecordingJobId='';
          if(recorded?.endedAt&&recorded.recordingState!=='running'&&(!selectedRun||selectedRun.id===job.runId)){
            const completed=await api.run(job.runId);
            // Do not replace a newer selection or an already-started run.
            if(recordingFinished(job)&&completed.id===job.runId&&(!selectedRun||selectedRun.id===completed.id)){
              selectedRun=completed;runTimeline=completed.timeline||[];reconciliation=completed.reconciliation||reconciliation;
            }
          }
        }catch(e){finishedRecordingJobId='';error=e.message;}
      }
      if (!['starting','running','pausing','paused','cancelling'].includes(job.status)) {
        events.close();
        generatorBusy=false;
        if(job.command==='smoke'){
          if(selectedRun?.id===`smoke:${job.id}`){const elapsed=job.progress.control?.active_seconds??job.progress.elapsed_seconds;const flows=selectedRun.flows.map(f=>({...f,successfulTps:Number(((job.progress.flows[f.name]?.successes||0)/Math.max(1,elapsed)).toFixed(2))}));selectedRun={...selectedRun,flows,successfulTps:Number(flows.reduce((sum,f)=>sum+f.successfulTps,0).toFixed(2)),live:false,complete:true,endedAt:job.endedAt,loadState:job.status==='complete'?'completed':job.status,recordingState:'disabled'};}
          notice='';
        }
        else if(job.command==='reconcile') reconciliation=job.result;
        else {await refreshRuns();if(!selectedRun || selectedRun.id===job.runId){const latest=runs.find(r=>r.id===job.runId) || runs[0];if(latest)await selectRun(latest);}}
        if(job.error&&!/k6 thresholds failed/i.test(job.error))error=job.error;
      }
    },(state,at)=>{connection=state;lastUpdate=at;});
  }

  async function reconcile() {
    if(!selectedRun||reconciling)return;
    error='';reconciling=true;reconciliation=null;
    const runId=selectedRun.id;
    try{
      reconciliationJob=await api.reconcile(runId,allowIncomplete);
      watchReconciliation(runId);
    }catch(e){error=e.message;reconciling=false;}
  }
  function watchReconciliation(runId){
      reconciliationEvents?.close();
      reconciliationEvents=watchJobStream(reconciliationJob.id,value=>{
        reconciliationJob=value;
        if(!['starting','running','cancelling'].includes(reconciliationJob.status)){
          reconciliationEvents.close();reconciling=false;
          if(selectedRun?.id===runId){reconciliation=reconciliationJob.result;if(reconciliationJob.error)error=reconciliationJob.error;}
        }
      },(state,at)=>{connection=state;lastUpdate=at;});
  }

  const modeLabel = (mode) => mode === 'tenancy' ? 'Tenancy' : 'Single';
  const loadLabel = (mode,tps) => `${modeLabel(mode)} · ${tps ?? '—'} TPS`;
  function formatDate(value) { return value ? new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(value)) : 'Unknown'; }
  const fixtureLabel = fixture => `${fixture.id.slice(0,8)} · ${new Intl.DateTimeFormat(undefined, {dateStyle:'medium'}).format(new Date(fixture.created_at))}`;
  const formatLatency=value=>value==null?'—':Number(value.toFixed(1));
  const series = (points,key) => (points || []).map(p=>({t:p.t,value:p[key]}));
  $: isSmoke=selectedRun?.kind==='smoke';
  $: thresholdFailure=loadState==='failed'&&/k6 thresholds failed/i.test(viewJob?.error||job?.error||'');
  $: timelineStopping=live&&stopping;
  $: timelineStatus=selectedRun?{state:timelineStopping?'stopping':loadState,stopping:timelineStopping,stateLabel:timelineStopping?'Stopping…':isSmoke?(live?'Smoke':loadState==='completed'?'Smoke passed':'Smoke failed'):thresholdFailure?'Checks failed':null,reason:thresholdFailure?'HTTP failure threshold exceeded':null,live,smoke:isSmoke,label:isSmoke?`${smokeRps} TPS verification`:`${selectedRun.offeredTps} TPS`,clock:isSmoke?`${smokeDuration}s`:`${selectedRun.elapsedSeconds} / ${selectedRun.durationSeconds}s active · ${Math.round(selectedRun.wallSeconds)}s elapsed${live?` · ${job.progress?.control?.in_flight ?? 0} in flight`:''}`,pending:!!pending,faultActive:!!faults.active,cancel,pause:()=>control('pause'),resume:()=>control('resume'),runAgain,resultsUrl:`/results?run=${encodeURIComponent(selectedRun.id)}`} : null;
  $: previewStatus={state:stopping?'stopping':smokeActive?'running':pending?'starting':'idle',stateLabel:stopping?'Stopping…':smokeActive?'Smoke':pending?'Starting…':'Ready',live:true,smoke:smokeActive,stopping,label:smokeActive?`${smokeRps} TPS verification`:'',clock:smokeActive?`${smokeDuration}s`:`${shapePreview.duration}s · up to ${shapePreview.peak} TPS`,pending:!!pending,startBlocked:!!startBlocker,cancel,startLoad:()=>start('load'),smokeTest:()=>smokeDialog.showModal()};
  async function cancel() {
    if(!job || stopping)return;
    const id=job.id;
    stopRequestedJobId=id;error='';
    try{
      const cancelled=await api.cancel(id);
      // A late acknowledgement must not overwrite a newer terminal stream update.
      if(job?.id===id && activeJob)job=cancelled;
    }catch(e){if(stopRequestedJobId===id)stopRequestedJobId='';error=e.message;}
  }

  async function control(action){
    if(pending)return;pending=action;error='';
    try{job=await api.control(job.id,action);}catch(e){error=e.message;}finally{pending='';}
  }
  async function fireFault(){
    if(pending)return;pending='fault';error='';
    try{faults=await api.fireFault({...faultDraft,jobId:job.id});window.dispatchEvent(new Event('console-fault-state'));faultDialog.close();faultDraft.confirm='';}catch(e){error=e.message;}finally{pending='';}
  }
  async function restoreFault(){try{faults=await api.restoreFault();window.dispatchEvent(new Event('console-fault-state'));}catch(e){error=e.message;}}

</script>
<svelte:head><title>{tab==='performance'?'Performance':'Results'} · Testbench</title></svelte:head>
<dialog class="fault-dialog" bind:this={faultDialog} aria-labelledby="fault-title"><form onsubmit={e=>{e.preventDefault();fireFault();}}><div class="section-head"><h1 id="fault-title">Create fault</h1><button type="button" class="icon-button" aria-label="Close fault" onclick={()=>faultDialog.close()}><Icon name="close"/></button></div>{#if error}<div class="message error" role="alert">{error}</div>{/if}<label>Target<select bind:value={faultDraft.targetId} onchange={e=>{faultDraft.action=faults.targets.find(t=>t.id===e.currentTarget.value)?.actions[0] || 'outage';faultDraft.confirm='';}}>{#each faults.targets as target}<option value={target.id}>{target.label}</option>{/each}</select></label>{#if faultDraft.action==='outage'}<label>Downtime · seconds<input type="number" min="1" max="300" bind:value={faultDraft.durationSeconds}/></label>{/if}<p>{faultTarget?.adapter==='simulated'?'Simulation does not change infrastructure.':faultDraft.action==='failover'?'Triggers managed failover and verifies readiness.':'Stops the service, waits for the configured downtime, then restores it and verifies recovery.'}</p><label>Type “{faultDraft.action} {faultDraft.targetId}” to confirm<input autocomplete="off" bind:value={faultDraft.confirm}/></label><div class="actions"><button type="button" class="secondary" onclick={()=>faultDialog.close()}>Cancel</button><button disabled={!!pending || faultDraft.confirm!==`${faultDraft.action} ${faultDraft.targetId}`}>Apply fault</button></div></form></dialog>

<dialog class="smoke-dialog" bind:this={smokeDialog} aria-labelledby="smoke-title"><form onsubmit={e=>{e.preventDefault();start('smoke');}}><div class="section-head"><div><p class="eyebrow">Quick verification</p><h2 id="smoke-title">Run smoke test</h2></div><button type="button" class="icon-button" aria-label="Close smoke test" onclick={()=>smokeDialog.close()}><Icon name="close"/></button></div><p class="smoke-intro">Confirm that the selected scenario can complete its flows before starting the full load.</p><div class="smoke-context"><Icon name="shield" size={18}/><div><strong>{executionProfile?.name || 'Selected scenario'}</strong><span>Uses ready fixtures · skips faults and response recording</span></div></div><div class="fields smoke-fields"><label>Traffic<div class="number-field"><input aria-label="Smoke test TPS" type="number" min="1" max="100" required bind:value={smokeRps}/><span>TPS</span></div></label><label>Duration<div class="number-field"><input aria-label="Smoke test duration" type="number" min="1" max="300" required bind:value={smokeDuration}/><span>sec</span></div></label></div><div class="actions"><button type="button" class="quiet" onclick={()=>smokeDialog.close()}>Cancel</button><button disabled={!!pending||!!startBlocker}><Icon name="performance"/> Run smoke test</button></div></form></dialog>
<main>
{#if error}<div class="message error" role="alert">{error}</div>{/if}
{#if notice}<div class="message" role="status">{notice}</div>{/if}
  {#if settings && tab === 'performance'}
    <div class="section-head"><div><h1>Performance</h1>{#if selectedRun}<p>{selectedRun.profile?.name ? `${selectedRun.profile.name} · ` : ''}{formatDate(selectedRun.startedAt)}</p>{:else}<p>Run scenarios and watch your system respond.</p>{/if}</div><label class="run-select">Load run<select value={selectedRun?.id || ''} onchange={(e) => { const run = runs.find((item) => item.id === e.currentTarget.value); if (run) selectRun(run); else if(job?.command==='load' && job.runId===e.currentTarget.value)selectRun({id:job.runId}); else newRun(); }}><option value="">New run · selected scenario</option>{#if selectedRun?.kind==='smoke'}<option value={selectedRun.id}>Smoke · {formatDate(selectedRun.startedAt)}</option>{/if}{#if job?.command==='load' && activeJob && job.runId && !runs.some(r=>r.id===job.runId)}<option value={job.runId}>Live · {loadLabel(job.mode,job.profile?.totalRps)}</option>{/if}{#each runs as run}<option value={run.id}>{run.profile?.name||'Load test'} · {formatDate(run.startedAt)}</option>{/each}</select></label></div>
    {#if !selectedRun}<div class="load-controls execution-inputs"><label>Scenario<select bind:value={selectedProfileId} onchange={chooseProfile} disabled={!profiles.length}>{#each profiles as profile}<option value={profile.id}>{profile.name} · {modeLabel(profile.mode)} · {profile.totalRps} TPS</option>{/each}</select></label><label>Fixture set<select bind:value={selectedFixtureId} onchange={()=>setQuery({fixtures:selectedFixtureId})} disabled={!compatibleFixtureSets.length}>{#each compatibleFixtureSets as fixture}<option value={fixture.id}>{fixtureLabel(fixture)}</option>{/each}</select></label><label>Execution<select bind:value={execution}><option value="local">Local worker</option><option value="distributed" >Distributed · auto</option></select>{#if execution==='distributed'}<small>{workers.filter(worker=>worker.id!=='local'&&worker.status==='idle').length} ready workers · {workers.filter(worker=>worker.id!=='local'&&worker.status==='idle').reduce((sum,worker)=>sum+worker.capacityTps,0)} TPS capacity</small>{/if}</label>{#if execution==='distributed'}<small>{#if planError}{planError} <button class="quiet" onclick={()=>window.dispatchEvent(new CustomEvent('console-settings',{detail:{section:'workers'}}))}>Connect workers</button>{:else if workerPlan}{workerPlan.map(w=>`${w.workerName}: ${w.offeredTps} TPS`).join(' · ')} <button class="quiet" onclick={previewWorkers}>Refresh allocation</button>{:else}Planning workers…{/if}</small>{/if}</div>{/if}
    {#if !selectedRun&&startBlocker&&!activeJob}<p class="blocker" role="status">{startBlocker} {#if !selectedFixtureId}<a href={`/fixtures?profile=${selectedProfileId}`}>Open Fixtures</a>{/if}</p>{/if}
    {#if viewJob?.workers?.length}<div class="worker-strip" aria-label="Distributed workers">{#each viewJob.workers as worker}<span><i class:complete={worker.status==='complete'} class:failed={worker.status==='failed'}></i>{worker.name} · {worker.offeredTps} TPS · {worker.status}</span>{/each}</div>{/if}
    {#if selectedRun}
      {@const filteredFlow=selectedRun.flows?.find(f=>f.name===flowFilter)}

      <div class="stats">
        <StatCard label="Successful TPS" value={filteredFlow?filteredFlow.successfulTps:selectedRun.successfulTps} caption={filteredFlow?`${filteredFlow.offeredTps} offered TPS`:`${selectedRun.iterations} flows`} series={series(flowFilter==='all'?chartPoints:chartPoints.map(p=>({...p,...p.flows?.[flowFilter]})),'successfulTps')} color="#398453"/>
        <StatCard label="Client-side latency" value={formatLatency(filteredFlow?filteredFlow.confirmP99:selectedRun.confirmP99)} unit="ms" caption="p99 request latency" series={series(flowFilter==='all'?chartPoints:chartPoints.map(p=>({...p,...p.flows?.[flowFilter]})),'p99')} color="#5277b8"/>
        <StatCard label="Server-side latency" value={formatLatency(filteredFlow?filteredFlow.serverP99:selectedRun.serverP99)} unit="ms" caption="p99 request latency" series={series(flowFilter==='all'?chartPoints:chartPoints.map(p=>({...p,...p.flows?.[flowFilter]})),'serverP99')} color="#5277b8"/>
        <StatCard label="HTTP errors · all flows" value={selectedRun.displayErrorRate ?? selectedRun.errorRate} unit="%" caption={`${selectedRun.dropped} dropped flows`} series={series(chartPoints,'errors')} color={(selectedRun.displayErrorRate ?? selectedRun.errorRate) ? '#b67822' : '#398453'}/>
      </div>
      <LiveChart points={chartPoints} activity={activity} startedAt={selectedRun.startedAt} durationSeconds={selectedRun.durationSeconds} timeline={selectedRun.timeline || []} allPoints={selectedRun.telemetry || []} flows={selectedRun.flows} runStatus={timelineStatus} bind:flow={flowFilter} bind:metric={chartMetric}>
        {#if activeJob&&live&&connection!=='connected'}<ConnectionStatus state={connection} updatedAt={lastUpdate}/>{/if}
        {#if faults.active||faults.blocked}<div class="fault-notice" role="status">{faultNotice(faults,{live,stopping,activity})}{#if faults.blocked}<button class="secondary" disabled={faults.recovering} onclick={restoreFault}>Retry recovery</button>{/if}</div>{/if}
      </LiveChart>
      {#if selectedRun.flows.length>1}<details class="detail-section"><summary>Flow breakdown · {selectedRun.flows.length} flows</summary><div class="table-wrap"><table><thead><tr><th>Flow</th><th>Offered TPS</th><th>Successful TPS</th><th>Client-side latency</th><th>Server-side latency</th><th>Errors</th></tr></thead><tbody>{#each selectedRun.flows.filter(f=>flowFilter==='all'||f.name===flowFilter) as flow}<tr><td>{flow.label}</td><td>{flow.offeredTps}</td><td>{flow.successfulTps}</td><td>{flow.confirmP99 == null ? '—' : `${Number(flow.confirmP99.toFixed(1))} ms`}</td><td>{flow.serverP99 == null ? '—' : `${Number(flow.serverP99.toFixed(1))} ms`}</td><td>{flow.errors}</td></tr>{/each}</tbody></table></div></details>{/if}
    {:else}<LiveChart runStatus={previewStatus} durationSeconds={smokeActive?Number(smokeDuration):shapePreview.duration} timeline={smokeActive?runTimeline:loadTimeline(canonicalLoad(executionProfile),(executionProfile?.load?.stages || []).map((s,index)=>({index,target:{label:faults.targets.find(t=>t.id===s.fault?.targetId)?.label}})))} bind:metric={chartMetric}/>{/if}
    {#if activity.length}<details class="activity detail-section"><summary>Activity · {activity.length} events</summary>{#if activity.length}<ol>{#each activity as event (event.id)}<li><time>{new Date(event.at).toLocaleTimeString()}</time><span>{event.message}</span></li>{/each}</ol>{:else}<p>Run events will appear here.</p>{/if}</details>{/if}
    {#if viewJob || selectedRun?.diagnostics || (job && job.command!=='load')}<details class="diagnostics"><summary>Recording and diagnostics</summary>{#if selectedRun}<p>{selectedRun.persisted.toLocaleString()} responses persisted · {selectedRun.gaps} pending · recording {selectedRun.recordingState}</p>{/if}{#key viewJob?.id || selectedRun?.id || job?.id}<FollowingLog output={viewJob?.output ?? selectedRun?.diagnostics ?? job?.output ?? ''}/>{/key}</details>{/if}
  {/if}

  {#if tab === 'results'}
    <div class="section-head results-header"><div><h1>Results</h1>{#if selectedRun}<p>{(selectedRun.payments+(selectedRun.failedCreates||0)).toLocaleString()} outcomes · recording {selectedRun.recordingState}</p>{/if}</div><label class="run-select">Run<select value={selectedRun?.id || ''} disabled={reconciling} onchange={e=>{const run=runs.find(r=>r.id===e.currentTarget.value);if(run)selectRun(run);}}>{#if !runs.length}<option value="">No runs yet</option>{/if}{#each runs as run}<option value={run.id}>{run.profile?.name || 'Load test'} · {formatDate(run.startedAt)}</option>{/each}</select></label></div>
    {#if !runs.length}<section class="empty-state"><h2>No load-test runs yet</h2><p>Start a load test from Performance. Results and reconciliation will appear here when recording begins.</p><a class="button" href="/performance">Open Performance</a></section>{/if}
    {#if selectedRun}<RunSummary run={selectedRun}/><section class="reconcile results-comparison">
      <div class="section-head"><h2>Reconciliation</h2><button disabled={reconciling || (!selectedRun.complete && !allowIncomplete)} onclick={reconcile}>{reconciling?'Comparing…':'Reconcile'}</button></div>
      {#if selectedRun}
      {#if !selectedRun.complete}<label><input type="checkbox" bind:checked={allowIncomplete}/> Allow incomplete recording</label>{/if}
      <ReconciliationSummary result={reconciliation} job={reconciliationJob?.runId===selectedRun.id?reconciliationJob:null} running={reconciling}>
      {#if reconciliation||reconciling}{#key selectedRun.id}<PaymentChart run={selectedRun} result={reconciliation} revision={reconciliation?.comparedAt||reconciliationJob?.progress?.compared||0} running={reconciling}/>{/key}{:else}<p>Check recorded outcomes against the configured database.</p>{/if}
      </ReconciliationSummary>{#if reconciling&&connection!=='connected'}<ConnectionStatus state={connection} updatedAt={lastUpdate}/>{/if}
      {#if reconciliation?.transport?.attempts&&reconciliation.accounted==null}
      <details class="discrepancies"><summary>Transport failures · {reconciliation.transport.attempts} attempts · {reconciliation.transport.unresolved} unresolved</summary>
      <p>{reconciliation.transport.found} payment IDs found · {reconciliation.transport.missing} not found. Found means the payment exists; its PostgreSQL status is shown below.</p>
      {#if reconciliation.transport.unresolved}<p>No payment ID was received for {reconciliation.transport.unresolved} attempts. Their server outcome cannot be established from this recording.</p>{/if}
      <div class="table-wrap"><table><thead><tr><th>Operation</th><th>Failed attempts</th><th>Without payment ID</th></tr></thead><tbody>{#each reconciliation.transport.byOperation as row}<tr><td>{row.operation}</td><td>{row.attempts}</td><td>{row.unresolved}</td></tr>{/each}</tbody></table></div>
      {#if reconciliation.transport.details.length}<div class="table-wrap"><table><thead><tr><th>Merchant / Payment</th><th>Failed attempts</th><th>PostgreSQL status</th><th>Outcome</th></tr></thead><tbody>{#each reconciliation.transport.details as row}<tr><td><small>{row.merchantId}</small><br/>{row.paymentId}</td><td>{row.attempts}</td><td>{row.databaseStatus ?? '—'}</td><td>{row.outcome==='found'?'Found':'Not found'}</td></tr>{/each}</tbody></table></div>{/if}
      {#if reconciliation.transport.truncated}<p>First 100 payment IDs shown. Full comparisons are saved in reconciliation.sqlite.</p>{/if}
      </details>
      {/if}

      {/if}
    </section>{/if}
    {#if selectedRun}<details class="result-details" bind:open={responsesOpen}><summary>Recorded responses</summary>{#if responsesOpen}{#key selectedRun.id}<ResponseBrowser runId={selectedRun.id} initial={Object.fromEntries($page.url.searchParams)}/>{/key}{/if}</details>{/if}
  {/if}
</main>

<style>
.stats{grid-template-columns:repeat(4,minmax(0,1fr))}.fault-notice{font-size:12px;color:var(--warning);padding:8px 0}.fault-notice button{margin-left:12px}
.blocker{font-size:12px;color:var(--warning);padding:0 2px}.blocker a{margin-left:8px}.worker-strip{display:flex;gap:8px;flex-wrap:wrap;margin:0 0 18px}.worker-strip span{display:flex;align-items:center;gap:6px;background:white;border:1px solid var(--border);border-radius:6px;padding:6px 10px;font-size:11px}.worker-strip i{width:6px;height:6px;border-radius:50%;background:var(--accent)}.worker-strip i.complete{background:var(--success)}.worker-strip i.failed{background:var(--danger)}.results-header .run-select{max-width:560px;width:48%}.results-comparison{padding:24px}.results-comparison>.section-head{margin-bottom:16px;padding-bottom:16px;border-bottom:1px solid var(--border)}.discrepancies{margin-top:20px;padding-top:18px;border-top:1px solid var(--border)}.discrepancies[open]>summary{margin-bottom:14px}.execution-inputs{gap:16px}.execution-inputs>small{grid-column:1/-1;font-size:11px}.activity ol{margin-top:14px}.smoke-dialog .section-head{align-items:flex-start;margin-bottom:12px}.smoke-dialog .eyebrow{margin:0 0 4px}.smoke-intro{margin:0 0 18px}.smoke-context{display:flex;align-items:flex-start;gap:12px;padding:14px;background:var(--accent-soft);border-radius:9px;color:var(--accent)}.smoke-context>div{display:flex;flex-direction:column;gap:3px;min-width:0}.smoke-context strong{font-size:13px;color:var(--ink);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.smoke-context span{font-size:11px;color:var(--muted)}.smoke-fields{margin-top:20px}.number-field{position:relative}.number-field input{padding-right:48px}.number-field span{position:absolute;right:12px;top:50%;transform:translateY(-50%);font-size:11px;color:var(--muted);pointer-events:none}.smoke-dialog .actions{justify-content:space-between}
@media(max-width:800px){.results-header .run-select{width:100%;max-width:none}.results-comparison>.section-head{align-items:flex-start;flex-wrap:wrap}.results-comparison{padding:16px}}
@media(max-width:900px){.stats{grid-template-columns:repeat(2,minmax(0,1fr))}}
</style>
