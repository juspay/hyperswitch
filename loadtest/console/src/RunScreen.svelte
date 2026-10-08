<script>
  import { onMount } from 'svelte';
  import { api } from './api.js';
  import StatCard from './StatCard.svelte';
  import {loadShape,loadTimeline} from './load-shape.mjs';
  import LiveChart from './LiveChart.svelte';
  import {activeTime,eventActiveTime,compactSamples} from './chart-timeline.mjs';
  import FollowingLog from './FollowingLog.svelte';
  import ReconciliationSummary from './ReconciliationSummary.svelte';
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
  let notice = '';
  let error = '';
  let settings = structuredClone(data.settings);
  let savedSettings = JSON.stringify(settings);
  $: shapePreview=previewShape(executionProfile || settings);
  function previewShape(s){try{return s.loadShape==='staged'?loadShape({start_rps:Number(s.startRps),stages:JSON.parse(s.stagesJson)}):{peak:s.totalRps,duration:s.durationSeconds};}catch{return {peak:'—',duration:'—'};}}
  $: dirty = JSON.stringify(settings) !== savedSettings;
  $: activeJob = ['starting','running','pausing','paused','cancelling'].includes(job?.status);
  let runs = data.runs;
  let selectedRun = data.run || null;
  let runTimeline = data.run?.timeline || [];
  let job = data.job;
  let events;
  let faultDialog;
  let faults={targets:[],active:null,blocked:false};
  let faultDraft={targetId:'demo',action:'outage',durationSeconds:10,confirm:''};
  let flowFilter='all';
  let chartMetric='tps';
  let replaySeconds=null;
  let replayPlaying=false;
  $: viewJob=job?.command==='load' && (!selectedRun || job.runId===selectedRun.id) ? job : null;
  $: loadState=viewJob?.loadState || (selectedRun?'completed':'idle');
  $: live=selectedRun?.live && activeJob && job?.runId===selectedRun.id;
  $: chartPoints=(selectedRun?.telemetry || []).filter(p=>replaySeconds==null||live||activeTime(p)<=replaySeconds);
  $: shownPoint=chartPoints.at(-1);
  $: faultTarget=faults.targets.find(t=>t.id===faultDraft.targetId);
  $: activity=(live?job?.events:selectedRun?.activity) || viewJob?.events || [];
  $: shownActivity=activity.filter(e=>replaySeconds==null||live||eventActiveTime(e,selectedRun.startedAt,selectedRun.telemetry || [])<=replaySeconds);

  let reconciliationJob=data.reconciliationJob || null,reconciling=['starting','running','cancelling'].includes(data.reconciliationJob?.status),reconciliationEvents;
  let reconciliation = selectedRun?.reconciliation || null;
  let history = null;
  let allowIncomplete = false;

  const blankPanel = () => ({ title: '', url: '' });
  let panelDraft = blankPanel();
  let addingPanel = false;

  onMount(() => {
    if(job?.command==='load') watchJob();
    if(reconciling)watchReconciliation(selectedRun.id);
    const connections=async()=>{try{settings=await api.settings();faults=await api.faults();if(!faults.targets.some(t=>t.id===faultDraft.targetId)){faultDraft.targetId=faults.targets[0]?.id||'';faultDraft.action=faults.targets[0]?.actions[0]||'outage';}fixtureSets=await api.fixtures();if(!fixtureSets.some(f=>f.id===selectedFixtureId&&f.compatible&&f.status==='ready'))selectedFixtureId='';error='';}catch(e){error=e.message;}};window.addEventListener('console-connections-saved',connections);
    const polling=setInterval(async()=>{try{faults=await api.faults();const r=await fetch('/api/jobs');if(r.ok){const active=await r.json();generatorBusy=active!=null && active.command!=='reconcile';if(active?.command==='load'&&active.id!==job?.id){job=active;watchJob();}}}catch{}},3000);
    api.faults().then(value=>{faults=value;faultDraft.targetId=value.targets[0]?.id || '';faultDraft.action=value.targets[0]?.actions[0] || 'outage';}).catch(e=>error=e.message);
    const replay=setInterval(()=>{if(replayPlaying&&!live){const end=selectedRun?.telemetry?.length?activeTime(selectedRun.telemetry.at(-1)):0;replaySeconds=Math.min(end,(replaySeconds ?? 0)+1);if(replaySeconds>=end)replayPlaying=false;}},1000);
    return () => {events?.close();reconciliationEvents?.close();clearInterval(replay);clearInterval(polling);window.removeEventListener('console-connections-saved',connections);};
  });

  async function refreshRuns() { runs = await api.runs(); }

  async function selectRun(run) {
    error = ''; reconciliation = null;
    history = null; replaySeconds=null; replayPlaying=false;flowFilter='all';
    try { selectedRun = await api.run(run.id); runTimeline=selectedRun.timeline || []; reconciliation = selectedRun.reconciliation || null;if(tab==='results'){reconciliationEvents?.close();reconciliationJob=await api.reconciliationJob(run.id);reconciling=['starting','running','cancelling'].includes(reconciliationJob?.status);if(reconciling)watchReconciliation(run.id);}if(typeof window!=='undefined'){const u=new URL(window.location.href);u.searchParams.set('run',run.id);window.history.replaceState(window.history.state,'',u);}}
    catch (e) { error = e.message; }
  }

  let generatorBusy=data.busy || false;
  let profiles=data.profiles || [],fixtureSets=data.fixtures || [];
  let selectedProfileId=data.profileId || profiles[0]?.id || '';
  let selectedFixtureId=data.fixtureSetId || '';
  $: executionProfile=profiles.find(p=>p.id===selectedProfileId);
  function chooseProfile(){selectedFixtureId='';selectedRun=null;goto(`/performance?profile=${encodeURIComponent(selectedProfileId)}`,{replaceState:true,noScroll:true});}
  async function start(command) {
    if(pending||activeJob||generatorBusy)return;
    if(!executionProfile||!selectedFixtureId){error='Select a saved scenario and ready fixtures';return;}
    pending=command;error='';
    runTimeline=loadTimeline(executionProfile.load,(executionProfile.load.stages || []).map((s,index)=>({index,target:{label:faults.targets.find(t=>t.id===s.fault?.targetId)?.label}})));
    try{job=await api.start('load',{mode:executionProfile.mode,profileId:executionProfile.id,version:executionProfile.version,fixtureSetId:selectedFixtureId});selectedRun=null;replaySeconds=null;replayPlaying=false;flowFilter='all';watchJob();goto(`/performance?run=${encodeURIComponent(job.runId)}&profile=${encodeURIComponent(selectedProfileId)}`,{replaceState:true,noScroll:true});}
    catch(e){error=e.message;}finally{pending='';}
  }

  function watchJob() {
    events?.close();
    events = new EventSource(`/api/jobs/${encodeURIComponent(job.id)}/events`);
    events.addEventListener('progress', async (event) => {
      job = JSON.parse(event.data);
      if(job.faults)faults=job.faults;
      const profile=job.profile || settings;
      if(job.runId && (!selectedRun || selectedRun.id === job.runId)) {
        const p=job.progress;
        let telemetry=selectedRun?.telemetry || [];
        if(p.trend && (!telemetry.length || p.trend.t>telemetry.at(-1).t)) {
          telemetry=compactSamples([...telemetry,p.trend]);
        }
        if(p.elapsed_seconds!=null) selectedRun={...(selectedRun || {}),id:job.runId,mode:job.mode,startedAt:job.startedAt,profile:job.profile,
          telemetry,timeline:job.profile?.timeline || selectedRun?.timeline || runTimeline,
          activity:job.events,activeSeconds:p.control?.active_seconds ?? p.elapsed_seconds,loadState:job.loadState,confirmP99:p.p99_ms ?? null,
          live:true,complete:false,recordingState:'running',durationSeconds:profile.durationSeconds,
          elapsedSeconds:Number((p.control?.active_seconds ?? p.elapsed_seconds).toFixed(1)),iterations:p.iterations,successfulTps:Number(p.successful_tps.toFixed(2)),
          offeredTps:profile.totalRps,errorRate:p.http_attempts?Number((p.http_errors/p.http_attempts*100).toFixed(2)):0,
          dropped:p.dropped,enqueued:p.recorder.enqueued,persisted:p.recorder.persisted,gaps:p.recorder.enqueued-p.recorder.persisted,
          flows:profile.scenarios.map(f=>({name:f.name,label:flowCatalog.find(c=>c[0]===f.scenario)?.[1] || f.name,
            offeredTps:profile.totalRps*f.weight/100,successfulTps:Number((p.flows[f.name]?.tps || 0).toFixed(2)),
            confirmP99:p.flows[f.name]?.p99_ms ?? null,confirmP90:p.flows[f.name]?.p90_ms ?? null,errors:p.flows[f.name]?.failures || 0}))};
      }
      if (!['starting','running','pausing','paused','cancelling'].includes(job.status)) {
        events.close();
        if(job.command==='reconcile') reconciliation=job.result;
        else {await refreshRuns();if(!selectedRun || selectedRun.id===job.runId){const latest=runs.find(r=>r.id===job.runId) || runs[0];if(latest)await selectRun(latest);}}
        if(job.error)error=job.error;
      }
    });
  }

  async function addPanel() {
    if (!panelDraft.title.trim() || !panelDraft.url.trim()) return;
    try {settings.grafana.panels = await api.panels([...settings.grafana.panels, { ...panelDraft }]);panelDraft = blankPanel(); addingPanel = false;}
    catch(e){error=e.message;}
  }

  async function removePanel(index) {
    try{settings.grafana.panels = await api.panels(settings.grafana.panels.filter((_, i) => i !== index));}catch(e){error=e.message;}
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
      reconciliationEvents=new EventSource(`/api/jobs/${reconciliationJob.id}/events`);
      reconciliationEvents.addEventListener('progress',event=>{
        reconciliationJob=JSON.parse(event.data);
        if(!['starting','running','cancelling'].includes(reconciliationJob.status)){
          reconciliationEvents.close();reconciling=false;
          if(selectedRun?.id===runId){reconciliation=reconciliationJob.result;if(reconciliationJob.error)error=reconciliationJob.error;}
        }
      });
  }

  const modeLabel = (mode) => mode === 'tenancy' ? 'Tenancy' : 'Single';
  const loadLabel = (mode,tps) => `${modeLabel(mode)} · ${tps ?? '—'} TPS`;
  function formatDate(value) { return value ? new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(value)) : 'Unknown'; }
  function displayFlow(flow){if(replaySeconds==null||live)return flow;const point=shownPoint?.flows?.[flow.name];return {...flow,successfulTps:Number((point?.tps || 0).toFixed(2)),confirmP99:point?.p99 ?? null,errors:point?.failures ?? 0};}
  const formatLatency=value=>value==null?'—':Number(value.toFixed(1));
  const series = (points,key) => (points || []).map(p=>({t:p.t,value:p[key]}));
  async function historyPage(after=0) {try{history=await api.requests(selectedRun.id,after);}catch(e){error=e.message;}}
  async function cancel() {try{job=await api.cancel(job.id);}catch(e){error=e.message;}}

  async function control(action){
    if(pending)return;pending=action;error='';
    try{job=await api.control(job.id,action);}catch(e){error=e.message;}finally{pending='';}
  }
  async function fireFault(){
    if(pending)return;pending='fault';error='';
    try{faults=await api.fireFault({...faultDraft,jobId:job.id});faultDialog.close();faultDraft.confirm='';}catch(e){error=e.message;}finally{pending='';}
  }
  async function restoreFault(){try{faults=await api.restoreFault();}catch(e){error=e.message;}}

  function panelUrl(url) {
    const u=new URL(url,'http://placeholder');
    if(selectedRun){u.searchParams.set('from',String(selectedRun.startedAt));u.searchParams.set('to',selectedRun.endedAt?String(selectedRun.endedAt):'now');}
    return u.pathname+u.search;
  }
</script>
<svelte:head><title>{tab==='performance'?'Performance':'Results'} · Hyperswitch</title></svelte:head>
<dialog class="fault-dialog" bind:this={faultDialog} aria-labelledby="fault-title"><form onsubmit={e=>{e.preventDefault();fireFault();}}><div class="section-head"><div><h1 id="fault-title">Create fault</h1><p>Configured server targets only</p></div><button type="button" class="icon-button" aria-label="Close fault" onclick={()=>faultDialog.close()}>×</button></div>{#if error}<div class="message error" role="alert">{error}</div>{/if}<label>Target<select bind:value={faultDraft.targetId} onchange={e=>{faultDraft.action=faults.targets.find(t=>t.id===e.currentTarget.value)?.actions[0] || 'outage';faultDraft.confirm='';}}>{#each faults.targets as target}<option value={target.id}>{target.label}</option>{/each}</select></label><label>Action<select bind:value={faultDraft.action}>{#each faultTarget?.actions || [] as action}<option value={action}>{action}</option>{/each}</select></label>{#if faultDraft.action==='outage'}<label>Duration · seconds<input type="number" min="1" max="300" bind:value={faultDraft.durationSeconds}/></label>{/if}<p>{faultTarget?.adapter==='simulated'?'Simulation does not change infrastructure.':'This changes the configured infrastructure target. Recovery is recorded and verified.'}</p><label>Type “{faultDraft.action} {faultDraft.targetId}” to confirm<input autocomplete="off" bind:value={faultDraft.confirm}/></label><div class="actions"><button type="button" class="secondary" onclick={()=>faultDialog.close()}>Cancel</button><button disabled={!!pending || faultDraft.confirm!==`${faultDraft.action} ${faultDraft.targetId}`}>Apply fault</button></div></form></dialog>

<main>
{#if error}<div class="message error" role="alert">{error}</div>{/if}
  {#if settings && tab === 'performance'}
    {#if !selectedRun}<div class="load-controls execution-inputs"><label>Scenario<select bind:value={selectedProfileId} onchange={chooseProfile}>{#each profiles as profile}<option value={profile.id}>{profile.name} · {modeLabel(profile.mode)} · {profile.totalRps} TPS · v{profile.version}</option>{/each}</select></label><label>Fixture set<select bind:value={selectedFixtureId}><option value="">Choose ready fixtures</option>{#each fixtureSets.filter(f=>f.status==='ready' && f.compatible && f.mode===executionProfile?.mode && f.merchant_count===executionProfile?.merchantCount) as fixture}<option value={fixture.id}>{modeLabel(fixture.mode)} · {fixture.merchant_count} {fixture.merchant_count===1?'merchant':'merchants'} · {fixture.id.slice(0,8)}</option>{/each}</select></label><a class="quiet" href={`/fixtures?profile=${selectedProfileId}`}>Manage fixtures</a></div>{/if}

    <div class="section-head"><div><h1>Performance</h1>{#if selectedRun}<p>{selectedRun.profile?.name ? `${selectedRun.profile.name} · v${selectedRun.profile.version} · ` : ''}{formatDate(selectedRun.startedAt)} · {loadLabel(selectedRun.mode,selectedRun.offeredTps)} · {selectedRun.durationSeconds}s · {selectedRun.live?loadState:selectedRun.recordingState}</p>{/if}</div><label class="run-select">Load run<select value={selectedRun?.id || ''} onchange={(e) => { const run = runs.find((item) => item.id === e.currentTarget.value); if (run) selectRun(run); else if(job?.command==='load' && job.runId===e.currentTarget.value)selectRun({id:job.runId}); else {selectedRun=null;replaySeconds=null;replayPlaying=false;const u=new URL(window.location.href);u.searchParams.delete('run');window.history.replaceState(window.history.state,'',u);} }}><option value="">New run · selected scenario</option>{#if job?.command==='load' && activeJob && job.runId && !runs.some(r=>r.id===job.runId)}<option value={job.runId}>Live · {loadLabel(job.mode,job.profile?.totalRps)}</option>{/if}{#each runs as run}<option value={run.id}>{formatDate(run.startedAt)} · {loadLabel(run.mode,run.offeredTps)}</option>{/each}</select></label></div>
    {#if loadState==='idle' && executionProfile?.load?.stages?.some(stage=>stage.fault)}<div class="message" role="status">Starting this scenario executes its saved faults: {executionProfile.load.stages.filter(stage=>stage.fault).map(stage=>`${faults.targets.find(t=>t.id===stage.fault.targetId)?.label || stage.fault.targetId} · ${stage.fault.action} · ${stage.duration_seconds}s`).join(' / ')}.</div>{/if}
    <div class="run-toolbar">
      <div><span class="state-badge" class:running={loadState==='running'} class:paused={loadState==='paused'} role="status">{loadState[0].toUpperCase()+loadState.slice(1)}</span><strong>{loadLabel(selectedRun?.mode || executionProfile?.mode || settings.mode,selectedRun?.offeredTps ?? shapePreview.peak)}</strong><span class="run-clock">{selectedRun?.elapsedSeconds ?? 0} / {selectedRun?.durationSeconds ?? shapePreview.duration}s active{live ? ` · ${job.progress?.control?.in_flight ?? 0} flows in flight` : ''}</span></div>
      <div class="actions">
        {#if loadState==='idle'}<button disabled={!!pending || activeJob || generatorBusy || !selectedFixtureId} onclick={()=>start('load')}>▶ Start</button>{/if}
        {#if loadState==='running'}<button disabled={!!pending || faults.active!=null} onclick={()=>control('pause')}>Ⅱ Pause</button>{/if}
        {#if loadState==='pausing'}<button disabled>Draining active flows…</button>{/if}
        {#if loadState==='paused'}<button disabled={!!pending} onclick={()=>control('resume')}>▶ Resume</button>{/if}
        {#if ['running','paused'].includes(loadState)}<button class="secondary" disabled={!!pending} onclick={cancel}>Stop</button>{/if}
        {#if loadState==='running'}<button class="secondary" disabled={!!pending || !!faults.active || faults.blocked} onclick={()=>faultDialog.showModal()}>Create fault</button>{/if}
      </div>
    </div>
    {#if faults.active || faults.blocked}<div class="message" role="status">{faults.active?.simulated?'Simulation · ':''}{faults.active?.target || 'Fault recovery'} · {faults.active?.status || 'pending'}{#if faults.blocked}<button class="secondary" disabled={faults.recovering} onclick={restoreFault}>{faults.recovering?'Recovering…':'Retry recovery'}</button>{/if}</div>{/if}
    {#if false}<div class="message">{job.command} · {job.status}<button class="secondary" onclick={cancel}>Stop</button></div>{/if}
    {#if selectedRun}
      <div class="stats">
        <StatCard label="Successful TPS" value={replaySeconds==null?selectedRun.successfulTps:Number((shownPoint?.tps || 0).toFixed(2))} caption={`${selectedRun.offeredTps} offered · ${replaySeconds==null?selectedRun.iterations:shownPoint?.successes ?? 0} ${replaySeconds==null?'flows':'successful flows'}`} series={series(chartPoints,'tps')} color="#398453"/>
        <StatCard label="Confirm p99" value={formatLatency(replaySeconds==null?selectedRun.confirmP99:shownPoint?.p99)} unit="ms" caption="Payment confirmation latency" series={series(chartPoints,'p99')} color="#5277b8"/>
        <StatCard label="HTTP errors" value={replaySeconds==null?selectedRun.errorRate:Number((shownPoint?.errors || 0).toFixed(2))} unit="%" caption={`${selectedRun.dropped} dropped flows`} series={series(chartPoints,'errors')} color={selectedRun.errorRate ? '#b67822' : '#398453'}/>
        <StatCard tooltip="Response events waiting for SQLite persistence. Temporary backlog is normal; it must reach zero before recording completes." label="Pending writes" value={replaySeconds==null?selectedRun.gaps:shownPoint?.backlog ?? 0} caption={`${selectedRun.persisted} persisted · ${selectedRun.complete ? 'Complete' : selectedRun.recordingState}`} series={series(chartPoints,'backlog')} color="#5277b8"/>
      </div>
      <div class="chart-controls">{#if !live && selectedRun.telemetry?.length}<div class="replay-controls"><button class="secondary" onclick={()=>{if(!replayPlaying && (replaySeconds==null||replaySeconds>=activeTime(selectedRun.telemetry.at(-1))))replaySeconds=0;replayPlaying=!replayPlaying;}}>{replayPlaying?'Ⅱ Pause replay':'▶ Replay'}</button><input aria-label="Replay time" type="range" min="0" max={activeTime(selectedRun.telemetry.at(-1))} step="1" value={replaySeconds ?? activeTime(selectedRun.telemetry.at(-1))} oninput={e=>{replaySeconds=Number(e.currentTarget.value);replayPlaying=false;}}/><span>{Math.round(replaySeconds ?? activeTime(selectedRun.telemetry.at(-1)))}s</span><button class="quiet" onclick={()=>{replaySeconds=null;replayPlaying=false;}}>Full run</button></div>{/if}</div>
      <LiveChart points={chartPoints} activity={shownActivity} startedAt={selectedRun.startedAt} durationSeconds={selectedRun.durationSeconds} timeline={selectedRun.timeline || []} allPoints={selectedRun.telemetry || []} flows={selectedRun.flows} bind:flow={flowFilter} bind:metric={chartMetric}/>
      <div class="table-wrap"><table><thead><tr><th>Flow</th><th>Offered TPS</th><th>Successful TPS</th><th>Confirm p99</th><th>Errors</th></tr></thead><tbody>{#each selectedRun.flows as flow}{@const display=displayFlow(flow)}<tr><td>{display.label}</td><td>{display.offeredTps}</td><td>{display.successfulTps}</td><td>{display.confirmP99 == null ? '—' : `${Number(display.confirmP99.toFixed(1))} ms`}</td><td>{display.errors}</td></tr>{/each}</tbody></table></div>
    {:else}<LiveChart durationSeconds={shapePreview.duration} timeline={loadTimeline(executionProfile?.load || {},(executionProfile?.load?.stages || []).map((s,index)=>({index,target:{label:faults.targets.find(t=>t.id===s.fault?.targetId)?.label}})))} bind:metric={chartMetric}/><p class="muted">Select ready fixtures and start to fill the timeline.</p>{/if}
    <section class="activity"><h2>Activity</h2>{#if shownActivity.length}<ol>{#each shownActivity as event (event.id)}<li><time>{new Date(event.at).toLocaleTimeString()}</time><span>{event.message}</span></li>{/each}</ol>{:else}<p>Run events will appear here.</p>{/if}</section>
    {#if viewJob || selectedRun?.diagnostics || (job && job.command!=='load')}<details class="diagnostics"><summary>Diagnostics · k6 output</summary>{#key viewJob?.id || selectedRun?.id || job?.id}<FollowingLog output={viewJob?.output ?? selectedRun?.diagnostics ?? job?.output ?? ''}/>{/key}</details>{/if}
    <div class="section-head panels-head"><h2>Grafana</h2><button class="secondary" onclick={() => addingPanel = true}>Add panel</button></div>
    {#if addingPanel}<div class="panel-form"><label>Name<input bind:value={panelDraft.title} /></label><label>Panel URL<input type="url" bind:value={panelDraft.url} placeholder="Panel embed URL, or dashboard URL with viewPanel" /></label><div class="actions"><button class="secondary" onclick={() => addingPanel = false}>Cancel</button><button onclick={addPanel}>Add</button></div></div>{/if}
    <div class="panel-grid">{#each settings.grafana.panels as panel, index}<article class="grafana-panel"><div><h3>{panel.title}</h3><button class="icon-button" title="Remove panel" aria-label={`Remove ${panel.title} panel`} onclick={() => removePanel(index)}>×</button></div><iframe title={panel.title} src={panelUrl(panel.url)} loading="lazy"></iframe></article>{/each}</div>
  {/if}

  {#if tab === 'results'}
    <div class="section-head results-header"><div><h1>Results</h1>{#if selectedRun}<p>{loadLabel(selectedRun.mode,selectedRun.offeredTps)} · {selectedRun.payments} payments · recording {selectedRun.recordingState}</p>{/if}</div><label class="run-select">Run<select value={selectedRun?.id || ''} disabled={reconciling} onchange={e=>{const run=runs.find(r=>r.id===e.currentTarget.value);if(run)selectRun(run);}}>{#if !runs.length}<option value="">No runs yet</option>{/if}{#each runs as run}<option value={run.id}>{run.profile?.name || 'Load test'} · {loadLabel(run.mode,run.offeredTps)} · {formatDate(run.startedAt)}</option>{/each}</select></label></div>
    <section class="reconcile results-comparison">
      <div class="section-head"><div><h2>Reconciliation</h2>{#if !selectedRun}<p>Select a run to compare its recorded payments with PostgreSQL.</p>{/if}</div>{#if selectedRun}<div class="comparison-actions"><button class="quiet" onclick={()=>window.dispatchEvent(new Event('console-settings'))}>Database settings</button><button disabled={reconciling || (!selectedRun.complete && !allowIncomplete)} onclick={reconcile}>{reconciling?'Comparing…':reconciliation?'Compare again':'Reconcile'}</button></div>{/if}</div>
      {#if selectedRun}
      {#if !selectedRun.complete}<label><input type="checkbox" bind:checked={allowIncomplete}/> Allow incomplete recording</label>{/if}
      <ReconciliationSummary result={reconciliation} job={reconciliationJob?.runId===selectedRun.id?reconciliationJob:null} running={reconciling}/>
      {#if reconciliation?.transport?.attempts}
      <details class="discrepancies"><summary>Transport failures · {reconciliation.transport.attempts} attempts · {reconciliation.transport.unresolved} unresolved</summary>
      <p>{reconciliation.transport.found} payment IDs found · {reconciliation.transport.missing} not found. Found means the payment exists; its PostgreSQL status is shown below.</p>
      {#if reconciliation.transport.unresolved}<p>No payment ID was received for {reconciliation.transport.unresolved} attempts. Their server outcome cannot be established from this recording.</p>{/if}
      <div class="table-wrap"><table><thead><tr><th>Operation</th><th>Failed attempts</th><th>Without payment ID</th></tr></thead><tbody>{#each reconciliation.transport.byOperation as row}<tr><td>{row.operation}</td><td>{row.attempts}</td><td>{row.unresolved}</td></tr>{/each}</tbody></table></div>
      {#if reconciliation.transport.details.length}<div class="table-wrap"><table><thead><tr><th>Merchant / Payment</th><th>Failed attempts</th><th>PostgreSQL status</th><th>Outcome</th></tr></thead><tbody>{#each reconciliation.transport.details as row}<tr><td><small>{row.merchantId}</small><br/>{row.paymentId}</td><td>{row.attempts}</td><td>{row.databaseStatus ?? '—'}</td><td>{row.outcome==='found'?'Found':'Not found'}</td></tr>{/each}</tbody></table></div>{/if}
      {#if reconciliation.transport.truncated}<p>First 100 payment IDs shown. Full comparisons are saved in reconciliation.sqlite.</p>{/if}
      </details>
      {/if}
      {#if reconciliation?.discrepancies?.length}<details class="discrepancies"><summary>Discrepancies · {reconciliation.mismatch+reconciliation.missing}{reconciliation.truncated?' · first 200 shown':''}</summary><div class="table-wrap"><table><thead><tr><th>Merchant / Payment</th><th>Recorded status</th><th>PostgreSQL status</th><th>Outcome</th></tr></thead><tbody>{#each reconciliation.discrepancies as row}<tr><td><small>{row.merchantId}</small><br/>{row.paymentId}</td><td>{row.recordedStatus}</td><td>{row.databaseStatus ?? '—'}</td><td class:missing-outcome={row.outcome==='missing'} class:mismatch-outcome={row.outcome==='mismatch'}>{row.outcome==='missing'?'Not found':'Status mismatch'}{#if row.reason}<small class="discrepancy-reason">{row.reason}</small>{/if}</td></tr>{/each}</tbody></table></div></details>{/if}
      {/if}
    </section>
    {#if selectedRun?.acceptance}<details class="acceptance result-details"><summary>Acceptance · {selectedRun.acceptance.verdict}</summary><table><thead><tr><th>Check</th><th>Recorded</th><th>Limit</th><th>Result</th></tr></thead><tbody>{#each selectedRun.acceptance.checks as check}<tr><td>{check.label}</td><td>{check.actual ?? '—'}</td><td>{Number(check.limit.toFixed(2))}</td><td>{check.outcome}</td></tr>{/each}</tbody></table><p>Saved run criteria. Simulation verifies controls only; it does not demonstrate infrastructure resilience.</p></details>{/if}
    {#if selectedRun}<details class="result-details"><summary>Recorded responses</summary><div class="section-head panels-head"><h2>Responses</h2><button class="secondary" onclick={() => historyPage()}>Load responses</button></div>
    {#if history}<div class="table-wrap"><table><thead><tr><th>Operation</th><th>Payment</th><th>HTTP</th><th>Latency</th><th>Message</th></tr></thead><tbody>{#each history.rows as row}<tr><td>{row.operation}</td><td>{row.payment_id || '—'}</td><td>{row.api_status_code || 'Transport failure'}</td><td>{Math.round(row.latency_ms)} ms</td><td><details><summary>{row.message || 'Response'}</summary><pre>{row.error_body || row.status || '—'}</pre></details></td></tr>{/each}</tbody></table></div>{#if history.next}<button class="secondary" onclick={() => historyPage(history.next)}>Next page</button>{/if}{/if}
    </details>{/if}
  {/if}
</main>

<style>.results-header .run-select{max-width:600px;width:50%}.result-details{margin:16px 0;padding:16px;border:1px solid var(--border);border-radius:12px;background:white}.result-details>summary{cursor:pointer;font-size:13px;font-weight:600}.result-details table{margin-top:16px}.results-comparison{padding:24px}.results-comparison h2{margin:0 0 8px;font-size:16px}.results-comparison .section-head{margin-bottom:0}.comparison-actions{display:flex;gap:8px;flex-wrap:wrap}.discrepancies{margin-top:24px}.discrepancies summary{font-size:13px;font-weight:600;cursor:pointer;margin-bottom:12px}.discrepancies td{overflow-wrap:anywhere}.discrepancies small{color:#8793a5}.discrepancy-reason{display:block;max-width:260px;margin-top:4px;font-size:11px}.missing-outcome{color:#c43b36}.mismatch-outcome{color:#bd7c21}@media(max-width:800px){.results-header .run-select{width:100%;max-width:none}.results-comparison .section-head{flex-direction:column}.results-comparison{padding:16px}}</style>
