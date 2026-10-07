<script>
  import { onMount } from 'svelte';
  import { api } from './api.js';
  import StatCard from './StatCard.svelte';
  import FollowingLog from './FollowingLog.svelte';
  export let data;

  const flowCatalog = [
    ['cit_off_session', 'CIT off-session'], ['sdk_checkout', 'SDK checkout'],
    ['guest', 'Guest payment'], ['cit_on_session', 'CIT on-session'],
    ['ptv_on_session', 'Pay to vault · on-session'], ['ptv_off_session', 'Pay to vault · off-session'],
    ['cit_metadata_changed', 'Card metadata update'], ['mit', 'Merchant-initiated payment'], ['saved_card_checkout', 'Saved-card checkout']
  ];
  let tab = 'scenarios';
  let settingsDialog;
  let pending = '';
  let notice = '';
  let error = '';
  let settings = structuredClone(data.settings);
  let savedSettings = JSON.stringify(settings);
  $: dirty = JSON.stringify(settings) !== savedSettings;
  $: activeJob = ['starting','running','cancelling'].includes(job?.status);
  let runs = data.runs;
  let selectedRun = data.run || runs[0] || null;
  let job = data.job;
  let events;
  let reconciliation = selectedRun?.reconciliation || null;
  let history = null;
  let allowIncomplete = false;

  const blankPanel = () => ({ title: '', url: '' });
  let panelDraft = blankPanel();
  let addingPanel = false;

  onMount(() => {
    if(job) watchJob();
    return () => events?.close();
  });

  async function saveSettings() {
    if (pending) return;
    pending = 'save'; error = ''; notice = '';
    try { settings = await api.saveSettings(settings); savedSettings = JSON.stringify(settings); notice = 'Profile saved'; settingsDialog?.close(); await refreshRuns(); }
    catch (e) { error = e.message; }
    finally { pending = ''; }
  }

  async function refreshRuns() { runs = await api.runs(); }

  async function selectRun(run) {
    error = ''; reconciliation = null;
    history = null;
    try { selectedRun = await api.run(run.id); reconciliation = selectedRun.reconciliation || null; }
    catch (e) { error = e.message; }
  }

  function addFlow() {
    settings.scenarios = [...settings.scenarios, { name: `flow_${settings.scenarios.length + 1}`, merchant_path: 'non_modular', scenario: 'guest', weight: 0 }];
  }

  function removeFlow(index) { settings.scenarios = settings.scenarios.filter((_, i) => i !== index); }
  $: allocated = settings.scenarios.reduce((sum, item) => sum + Number(item.weight || 0), 0);

  async function start(command) {
    if (allocated !== 100) { error = 'Flow weights must total 100%'; return; }
    if (pending || activeJob) return;
    pending = command; error = ''; notice = '';
    try { settings = await api.saveSettings(settings); savedSettings = JSON.stringify(settings); job = await api.start(command, settings.mode); if(command==='load')selectedRun=null; tab = 'performance'; watchJob(); }
    catch (e) { error = e.message; }
    finally { pending = ''; }
  }

  function watchJob() {
    events?.close();
    events = new EventSource(`/api/jobs/${encodeURIComponent(job.id)}/events`);
    events.addEventListener('progress', async (event) => {
      job = JSON.parse(event.data);
      const profile=job.profile || settings;
      if(job.runId && (!selectedRun || selectedRun.id === job.runId)) {
        const p=job.progress;
        let telemetry=selectedRun?.telemetry || [];
        if(p.trend && (!telemetry.length || p.trend.t>telemetry.at(-1).t)) {
          telemetry=[...telemetry,p.trend].slice(-1800);
        }
        if(p.elapsed_seconds!=null) selectedRun={...(selectedRun || {}),id:job.runId,mode:job.mode,startedAt:job.startedAt,
          telemetry,
          live:true,complete:false,recordingState:'running',durationSeconds:profile.durationSeconds,
          elapsedSeconds:Math.round(p.elapsed_seconds),iterations:p.iterations,successfulTps:Number(p.successful_tps.toFixed(2)),
          offeredTps:profile.totalRps,errorRate:p.http_attempts?Number((p.http_errors/p.http_attempts*100).toFixed(2)):0,
          dropped:p.dropped,enqueued:p.recorder.enqueued,persisted:p.recorder.persisted,gaps:p.recorder.enqueued-p.recorder.persisted,
          flows:profile.scenarios.map(f=>({name:f.name,label:flowCatalog.find(c=>c[0]===f.scenario)?.[1] || f.name,
            offeredTps:profile.totalRps*f.weight/100,successfulTps:Number((p.flows[f.name]?.tps || 0).toFixed(2)),
            confirmP90:p.flows[f.name]?.p90_ms ?? null,errors:p.flows[f.name]?.failures || 0}))};
      }
      if (!['starting','running','cancelling'].includes(job.status)) {
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
    if (!selectedRun) return;
    error = ''; reconciliation = null;
    try { job = await api.reconcile(selectedRun.id,allowIncomplete); watchJob(); }
    catch (e) { error = e.message; }
  }

  const modeLabel = (mode) => mode === 'tenancy' ? 'Tenancy' : 'Single';
  const loadLabel = (mode,tps) => `${modeLabel(mode)} · ${tps ?? '—'} TPS`;
  function formatDate(value) { return value ? new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(value)) : 'Unknown'; }
  const series = (points,key) => (points || []).map(p=>({t:p.t,value:p[key]}));
  async function historyPage(after=0) {try{history=await api.requests(selectedRun.id,after);}catch(e){error=e.message;}}
  async function cancel() {try{job=await api.cancel(job.id);}catch(e){error=e.message;}}
  function panelUrl(url) {
    const u=new URL(url,'http://placeholder');
    if(selectedRun){u.searchParams.set('from',String(selectedRun.startedAt));u.searchParams.set('to',selectedRun.endedAt?String(selectedRun.endedAt):'now');}
    return u.pathname+u.search;
  }
</script>

<svelte:head><title>Hyperswitch / Load testing</title><meta name="description" content="Payment flow load testing" /></svelte:head>

<header>
  <a class="brand" href="/"><img src="/hyperswitch-logo.svg" alt="Hyperswitch" /><span>Load testing</span></a>
  <div class="header-actions"><button class="header-button" onclick={() => settingsDialog.showModal()}><span aria-hidden="true">⚙</span> Settings</button><form action="/logout" method="POST"><button class="header-button">Sign out <span aria-hidden="true">↗</span></button></form></div>
</header>

<dialog class="settings-dialog" bind:this={settingsDialog} aria-labelledby="settings-title">
  <section class="settings">
    <div class="section-head"><div><h1 id="settings-title">Settings</h1><p>Connections and infrastructure</p></div><button class="icon-button" aria-label="Close settings" onclick={() => settingsDialog.close()}>×</button></div>
    {#if error}<div class="message error" role="alert">{error}</div>{/if}
    <div class="settings-grid">
      <div>
        <h2>Services</h2>
        <div class="fields"><label>Router<input bind:value={settings.router} /></label><label>Payment methods<input bind:value={settings.modularPm} /></label><label>Router admin key<input type="password" bind:value={settings.adminApiKey} placeholder={settings.hasAdminApiKey ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Connector<input bind:value={settings.connector} /></label></div>
        <h2>Superposition</h2>
        <div class="fields"><label>Endpoint<input bind:value={settings.superposition.endpoint} /></label><label>Token<input type="password" bind:value={settings.superposition.token} placeholder={settings.superposition.hasToken ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Organization<input bind:value={settings.superposition.organizationId} /></label><label>Workspace<input bind:value={settings.superposition.workspaceId} /></label></div>
        <label>Defaults<textarea rows="5" bind:value={settings.superposition.defaultsJson}></textarea></label>
      </div>
      <div>
        <details class="advanced"><summary>Advanced load settings</summary><div class="fields"><label>Refresh wait · seconds<input type="number" min="0" bind:value={settings.superposition.refreshSeconds} /></label><label>Preallocated VUs / TPS<input type="number" min="0.1" step="0.1" bind:value={settings.preAllocatedVusMultiplier}/></label><label>Maximum VUs / TPS<input type="number" min="0.1" step="0.1" bind:value={settings.maxVusMultiplier}/></label></div></details>
        <h2>Grafana</h2><label>Base URL<input bind:value={settings.grafana.baseUrl} placeholder="http://localhost:3002" /></label>
        <p>{settings.grafana.hasIdentity ? 'Server credentials configured' : 'Set GRAFANA_TOKEN in the server .env to a Viewer service-account token, then restart the console.'}</p>
        <h2>Reconciliation database</h2>
        <div class="fields"><label>Host<input bind:value={settings.database.host} /></label><label>Port<input type="number" bind:value={settings.database.port} /></label><label>Database<input bind:value={settings.database.database} /></label><label>Read-only user<input bind:value={settings.database.user} /></label><label>Password<input type="password" bind:value={settings.database.password} placeholder={settings.database.hasPassword ? 'Configured · leave blank to keep' : 'Required'} /></label></div>
        <label class="checkbox"><input type="checkbox" bind:checked={settings.database.ssl}/> Require verified TLS</label>
      </div>
    </div>
    <div class="actions"><button class="secondary" onclick={() => settingsDialog.close()}>Close</button><button disabled={!!pending || !dirty} onclick={saveSettings}>{pending === 'save' ? 'Saving…' : 'Save settings'}</button></div>
  </section>
</dialog>

<nav aria-label="Console sections">
  {#each ['scenarios', 'performance', 'results'] as item}
    <button class:active={tab === item} onclick={() => tab = item}>{item[0].toUpperCase() + item.slice(1)}</button>
  {/each}
</nav>

<main>
  {#if error}<div class="message error" role="alert">{error}</div>{/if}
  {#if notice && notice !== 'Profile saved'}<div class="message">{notice}</div>{/if}
  {#if settings && tab === 'scenarios'}
    <div class="section-head"><div><h1>Scenarios</h1><p>{loadLabel(settings.mode,settings.totalRps)} · {settings.durationSeconds}s</p></div><button class="secondary" onclick={addFlow}><span aria-hidden="true">+</span> Add flow</button></div>
    <div class="load-controls"><label>Merchant mode<select bind:value={settings.mode}><option value="single">Single · 1 merchant</option><option value="tenancy">Tenancy · 1,500 merchants</option></select></label><label>Offered TPS<input type="number" min="1" bind:value={settings.totalRps}/></label><label>Duration <span>seconds</span><input type="number" min="1" bind:value={settings.durationSeconds}/></label></div>
    <div class="table-wrap flow-editor"><table><thead><tr><th>Flow</th><th>Flow name</th><th>Path</th><th>Weight</th><th></th></tr></thead><tbody>
      {#each settings.scenarios as flow, index}
        <tr><td><select aria-label={`${flow.name} flow type`} bind:value={flow.scenario}>{#each flowCatalog as choice}<option value={choice[0]}>{choice[1]}</option>{/each}</select></td><td><input aria-label="Flow name" bind:value={flow.name} /></td><td><select aria-label={`${flow.name} integration path`} bind:value={flow.merchant_path}><option value="modular">Modular</option><option value="non_modular">Non-modular</option></select></td><td><div class="stepper"><button class="step-button" aria-label={`Decrease ${flow.name} weight`} disabled={flow.weight <= 0} onclick={() => flow.weight = Math.max(0,Number(flow.weight || 0)-1)}>−</button><input aria-label={`${flow.name} weight percent`} class="weight" type="number" min="0" max="100" bind:value={flow.weight}/><span>%</span><button class="step-button" aria-label={`Increase ${flow.name} weight`} disabled={flow.weight >= 100} onclick={() => flow.weight = Math.min(100,Number(flow.weight || 0)+1)}>+</button></div></td><td><button class="icon-button" title="Remove flow" aria-label={`Remove ${flow.name} flow`} onclick={() => removeFlow(index)}>×</button></td></tr>
      {/each}
    </tbody></table></div>
    <div class="flow-footer"><div class="profile-status"><span class:invalid={allocated !== 100}>{allocated}% allocated{allocated !== 100 ? ' · Must total 100%' : ''}</span><span class="muted" role="status">{dirty ? 'Unsaved changes' : notice === 'Profile saved' ? '✓ Profile saved' : 'Profile saved'}</span></div><div class="actions"><button class="secondary" disabled={!dirty || !!pending} onclick={saveSettings}>{pending === 'save' ? 'Saving…' : 'Save profile'}</button><button class="secondary" disabled={!!pending || activeJob || allocated !== 100} onclick={() => start('fixtures')}>{pending === 'fixtures' ? 'Starting…' : 'Create fixtures'}</button><button disabled={!!pending || activeJob || allocated !== 100 || !(settings.totalRps > 0) || !(settings.durationSeconds > 0)} onclick={() => start('load')}>{pending === 'load' ? 'Starting…' : activeJob ? 'Job in progress' : 'Start load test'}</button></div></div>
  {/if}

  {#if settings && tab === 'performance'}
    <div class="section-head"><div><h1>Performance</h1>{#if selectedRun}<p>{formatDate(selectedRun.startedAt)} · {loadLabel(selectedRun.mode,selectedRun.offeredTps)} · {selectedRun.durationSeconds}s · {selectedRun.recordingState}</p>{/if}</div><label class="run-select">Load run<select value={selectedRun?.id || ''} onchange={(e) => { const run = runs.find((item) => item.id === e.currentTarget.value); if (run) selectRun(run); }}><option value="">Select</option>{#each runs as run}<option value={run.id}>{formatDate(run.startedAt)} · {loadLabel(run.mode,run.offeredTps)}</option>{/each}</select></label></div>
    {#if job}<div class="job"><div class="section-head"><strong>{job.command} · {loadLabel(job.mode,job.profile?.totalRps ?? selectedRun?.offeredTps)} · {job.status}</strong>{#if ['starting','running','cancelling'].includes(job.status)}<button class="secondary" onclick={cancel}>Stop</button>{/if}</div>{#key job.id}<FollowingLog output={job.output}/>{/key}</div>{/if}
    {#if selectedRun}
      <div class="stats">
        <StatCard label="Elapsed" value={selectedRun.elapsedSeconds} unit={` / ${selectedRun.durationSeconds}s`} caption={selectedRun.live ? 'Running' : selectedRun.complete ? 'Complete' : 'Incomplete'} series={series(selectedRun.telemetry,'t')} color="#68707c"/>
        <StatCard label="Successful TPS" value={selectedRun.successfulTps} caption={`${selectedRun.offeredTps} offered · ${selectedRun.iterations} flows`} series={series(selectedRun.telemetry,'tps')} color="#398453"/>
        <StatCard label="Errors" value={selectedRun.errorRate} unit="%" caption={`${selectedRun.dropped} dropped`} series={series(selectedRun.telemetry,'errors')} color={selectedRun.errorRate ? '#b67822' : '#398453'}/>
        <StatCard label="Recorded" value={selectedRun.persisted} caption={`${selectedRun.enqueued} attempts · ${selectedRun.gaps} ${selectedRun.live ? 'pending' : 'gaps'}`} series={series(selectedRun.telemetry,'recorded')} color="#5277b8"/>
      </div>
      <div class="table-wrap"><table><thead><tr><th>Flow</th><th>Offered TPS</th><th>Successful TPS</th><th>Confirm p90</th><th>Errors</th></tr></thead><tbody>{#each selectedRun.flows as flow}<tr><td>{flow.label}</td><td>{flow.offeredTps}</td><td>{flow.successfulTps}</td><td>{flow.confirmP90 == null ? '—' : `${flow.confirmP90} ms`}</td><td>{flow.errors}</td></tr>{/each}</tbody></table></div>
    {:else}<div class="empty">No runs found</div>{/if}
    <div class="section-head panels-head"><h2>Grafana</h2><button class="secondary" onclick={() => addingPanel = true}>Add panel</button></div>
    {#if addingPanel}<div class="panel-form"><label>Name<input bind:value={panelDraft.title} /></label><label>Panel URL<input type="url" bind:value={panelDraft.url} placeholder="Panel embed URL, or dashboard URL with viewPanel" /></label><div class="actions"><button class="secondary" onclick={() => addingPanel = false}>Cancel</button><button onclick={addPanel}>Add</button></div></div>{/if}
    <div class="panel-grid">{#each settings.grafana.panels as panel, index}<article class="grafana-panel"><div><h3>{panel.title}</h3><button class="icon-button" title="Remove panel" aria-label={`Remove ${panel.title} panel`} onclick={() => removePanel(index)}>×</button></div><iframe title={panel.title} src={panelUrl(panel.url)} loading="lazy"></iframe></article>{/each}</div>
  {/if}

  {#if tab === 'results'}
    <div class="section-head"><div><h1>Results</h1><p>Recorded responses and database status</p></div></div>
    <div class="results-grid"><section><div class="table-wrap"><table><thead><tr><th>Run</th><th>Mode</th><th>Offered TPS</th><th>Flows</th><th>Recording</th></tr></thead><tbody>{#each runs as run}<tr class:selected={selectedRun?.id === run.id}><td><button class="quiet" onclick={() => selectRun(run)}>{formatDate(run.startedAt)}</button></td><td>{modeLabel(run.mode)}</td><td>{run.offeredTps}</td><td>{run.iterations ?? '—'}</td><td>{run.recordingState}</td></tr>{/each}</tbody></table></div></section>
      <section class="reconcile"><h2>Reconciliation</h2>{#if selectedRun}<p>{loadLabel(selectedRun.mode,selectedRun.offeredTps)} · {selectedRun.payments} recorded payments</p>{#if !selectedRun.complete}<label><input type="checkbox" bind:checked={allowIncomplete}/> Allow incomplete recording</label>{/if}<button onclick={reconcile}>Compare with PostgreSQL</button>{#if job?.command==='reconcile'}<p role="status">{job.status} · {job.progress?.compared || 0} compared</p>{/if}{#if reconciliation}<p>{reconciliation.comparedAt}</p><div class="recon-stats"><span><strong>{reconciliation.match}</strong>Match</span><span><strong>{reconciliation.mismatch}</strong>Mismatch</span><span><strong>{reconciliation.missing}</strong>Missing</span></div>{#if reconciliation.discrepancies.length}<div class="table-wrap"><table><thead><tr><th>Payment</th><th>Recorded</th><th>Database</th></tr></thead><tbody>{#each reconciliation.discrepancies as row}<tr><td>{row.paymentId}</td><td>{row.recordedStatus}</td><td>{row.databaseStatus || 'Missing'}</td></tr>{/each}</tbody></table></div>{/if}{/if}{:else}<p>Select a run.</p>{/if}</section>
    </div>
    {#if selectedRun}<div class="section-head panels-head"><h2>Responses</h2><button class="secondary" onclick={() => historyPage()}>View responses</button></div>{/if}
    {#if history}<div class="table-wrap"><table><thead><tr><th>Operation</th><th>Payment</th><th>HTTP</th><th>Latency</th><th>Message</th></tr></thead><tbody>{#each history.rows as row}<tr><td>{row.operation}</td><td>{row.payment_id || '—'}</td><td>{row.api_status_code || 'Transport failure'}</td><td>{Math.round(row.latency_ms)} ms</td><td><details><summary>{row.message || 'Response'}</summary><pre>{row.error_body || row.status || '—'}</pre></details></td></tr>{/each}</tbody></table></div>{#if history.next}<button class="secondary" onclick={() => historyPage(history.next)}>Next page</button>{/if}{/if}
  {/if}
</main>

<footer>Router · {settings.router} / PM · {settings.modularPm}</footer>
