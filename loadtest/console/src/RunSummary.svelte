<script>
import {environmentLabel} from './environments.mjs';
  import ScenarioChecks from './ScenarioChecks.svelte';
  import {resultTestCases} from './result-test-cases.mjs';
  export let run;
  const number=(value,digits=2)=>value==null||!Number.isFinite(Number(value))?'—':Number(value).toLocaleString(undefined,{maximumFractionDigits:digits});
  const duration=value=>{
    if(value==null||!Number.isFinite(Number(value)))return '—';
    const total=Math.max(0,Math.round(Number(value))),hours=Math.floor(total/3600),minutes=Math.floor(total%3600/60),seconds=total%60;
    return [hours?`${hours}h`:'',minutes?`${minutes}m`:'',seconds||(!hours&&!minutes)?`${seconds}s`:''].filter(Boolean).join(' ');
  };
  const formatStartedAt=value=>{
    const date=new Date(value),day=date.getDate();
    const suffix=day%100>=11&&day%100<=13?'th':({1:'st',2:'nd',3:'rd'}[day%10]||'th');
    return `${day}${suffix} ${date.toLocaleString('en-GB',{month:'short'})}, ${date.getFullYear()} · ${date.toLocaleTimeString('en-US',{hour:'numeric',minute:'2-digit',second:'2-digit'})}`;
  };
  const labels={'HTTP errors (%)':'HTTP errors'};
  const units={'Successful TPS':' TPS','HTTP errors (%)':'%'};
  $: acceptance=run.acceptance;
  $: faultLabels=[...new Set((run.timeline||[]).filter(stage=>stage.fault).map(stage=>`${stage.fault.target==='PostgreSQL'?'DB':stage.fault.target} ${stage.fault.action==='failover'?'failover':stage.fault.action==='restart'?'restart':'outage'}`))];
  $: visibleChecks=run.scenarioChecks?resultTestCases(run.scenarioChecks).flatMap(s=>s.cases):[];
  $: checked=visibleChecks.filter(c=>c.id.startsWith('TC-')||c.id==='completion');
  $: passed=checked.filter(c=>c.outcome==='pass').length;
  $: caseIncomplete=visibleChecks.some(c=>['not_measured','not_run','incomplete'].includes(c.outcome));
  $: pending=run.live||!run.endedAt;
  $: verdict=run.scenarioChecks?(run.scenarioChecks.verdict==='pass'&&caseIncomplete?'Incomplete':({pass:'Passed',fail:'Failed',incomplete:'Incomplete',recorded:'Recorded',in_progress:'In progress',not_run:'Not run'}[run.scenarioChecks.verdict]||'Recorded')):pending?'In progress':!run.savedSettings?.limitsSaved?'Recorded':({pass:'Passed',fail:'Failed',incomplete:'Incomplete',unmeasured:'Unmeasured'}[acceptance?.verdict]||'Recorded');
  $: checks=(acceptance?.checks||[]).filter(check=>['Successful TPS','HTTP errors (%)'].includes(check.label)).map(check=>({...check,label:labels[check.label]||check.label,
    observed:number(check.actual)+(check.actual==null?'':units[check.label]||''),
    threshold:run.savedSettings?.limitsSaved?(check.label==='Successful TPS'?'≥ ':'≤ ')+number(check.limit)+(units[check.label]||''):'—',
    outcome:pending?'unmeasured':!run.savedSettings?.limitsSaved?'recorded':check.outcome}));
</script>

<section class="run-summary" aria-labelledby="run-summary-title">
  <div class="summary-head"><div><span class="summary-eyebrow">Run summary</span><h2 id="run-summary-title">{run.profile?.name||'Load test run'}</h2><p>{formatStartedAt(run.startedAt)} · {duration(run.activeSeconds??run.elapsedSeconds)} active · {duration(run.wallSeconds)} elapsed</p></div><div class="summary-status">{#if checked.length}<span class="check-count">{passed}/{checked.length} test cases passed</span>{/if}<span class="verdict" class:failed={verdict==='Failed'} class:passed={verdict==='Passed'}>{verdict}</span></div></div>
  <div class="configuration"><div class="summary-labels">{#if run.environment}<span class="summary-label environment">{environmentLabel(run.environment)}</span>{/if}<span class="summary-label">{number(run.offeredTps)} TPS peak</span>{#each faultLabels as label}<span class="summary-label fault">{label}</span>{/each}</div></div>
  {#if run.scenarioChecks}<ScenarioChecks summary={run.scenarioChecks} latencyPhases={run.serverLatencyPhases}/>{:else}
  <div class="table-wrap"><table><thead><tr><th>Metric</th><th class="numeric">Limit</th><th class="numeric">Observed</th><th>Verdict</th></tr></thead><tbody>
    {#each checks as check}<tr><td>{check.label}</td><td class="numeric muted">{check.threshold}</td><td class="numeric">{check.observed}</td><td><span class="check" class:failed={check.outcome==='fail'} class:passed={check.outcome==='pass'}>{check.outcome==='pass'?'Pass':check.outcome==='fail'?'Fail':'—'}</span></td></tr>{/each}
    <tr><td>Completed flows</td><td class="numeric muted">—</td><td class="numeric">{number(run.iterations,0)}</td><td class="muted">—</td></tr>
  </tbody></table></div>
  <h3 class="latency-title">Server-side latency</h3>
  <div class="table-wrap"><table class="latency-table"><thead><tr><th>State</th><th class="numeric">Samples</th><th class="numeric">p50 · ms</th><th class="numeric">p95 · ms</th><th class="numeric">p99 · ms</th></tr></thead><tbody>
    {#each run.serverLatencyPhases||[] as phase}<tr><td title={phase.state==='normal'?'Requests outside recorded outage and recovery windows':phase.state==='outage'?'Requests overlapping a vault outage or observed database unavailability':`Requests overlapping the first ${run.recoveryLatencySeconds||10} seconds after recovery, excluding outage requests`}>{phase.state==='normal'?'Normal':phase.state==='outage'?'Outage':`After recovery · first ${run.recoveryLatencySeconds||10}s`}</td><td class="numeric muted">{number(phase.count,0)}</td><td class="numeric">{number(phase.p50,1)}</td><td class="numeric">{number(phase.p95,1)}</td><td class="numeric">{number(phase.p99,1)}</td></tr>{:else}<tr><td colspan="5" class="muted">No successful latency samples recorded</td></tr>{/each}
  </tbody></table></div>
  {/if}
</section>

<style>
.summary-eyebrow{display:block;font-size:11px;color:var(--muted);margin-bottom:6px}.summary-head h2{font-weight:700}.configuration{display:flex;align-items:center;flex-wrap:wrap;gap:8px 16px}.summary-labels{display:flex;flex-wrap:wrap;gap:6px}.summary-label{display:inline-flex;align-items:center;background:var(--surface-muted);border-radius:5px;padding:3px 8px;font-weight:500}.summary-label.environment{color:var(--primary,#0668ff);background:var(--primary-soft,#edf4ff)}.summary-label.fault{color:#9b651b;background:#fff7e9}

.latency-title{margin:20px 0 12px;font-size:12px}.latency-table td:last-child,.latency-table th:last-child{width:auto;padding-left:16px}
.summary-status{display:flex;align-items:center;gap:12px}.check-count{font-size:11px;color:var(--muted);white-space:nowrap}@media(max-width:600px){.summary-status{align-items:flex-end;flex-direction:column;gap:6px}}
.run-summary{background:var(--surface);border:1px solid var(--border);border-radius:var(--radius);padding:24px;margin-bottom:16px}.summary-head{display:flex;align-items:flex-start;justify-content:space-between;gap:16px}.summary-head p,.configuration{font-size:11px;color:var(--muted);line-height:1.6}.summary-head p{margin:6px 0 0}.configuration{margin:12px 0 16px}.verdict{font-size:11px;font-weight:500;border-radius:5px;padding:5px 9px;background:var(--surface-muted);color:var(--muted);white-space:nowrap}.verdict.failed{background:var(--danger-soft);color:var(--danger)}.verdict.passed{background:var(--success-soft);color:var(--success)}.numeric{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}.muted{color:var(--muted)}.check{font-size:11px;color:var(--muted)}.check.failed{color:var(--danger);font-weight:500}.check.passed{color:var(--success)}.table-wrap{border-top:1px solid var(--border)}td,th{padding-top:10px;padding-bottom:10px}td:last-child,th:last-child{width:90px;padding-left:24px}@media(max-width:600px){.run-summary{padding:16px}.configuration{margin-top:10px}table{min-width:460px}}
</style>
