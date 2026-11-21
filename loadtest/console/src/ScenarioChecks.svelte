<script>
  import {resultTestCases,formatTestCaseValue,formatTestCaseLimit} from './result-test-cases.mjs';
  export let summary;
  export let latencyPhases=null;
  $: sections=resultTestCases(summary);
  $: states=(summary.states??latencyPhases??[]).filter(s=>s.count>0||s.windowExpected);
  const latencyCheck=check=>/^p(50|95|99)$/.test(check.id);
  const periodMetrics=new Set(['tps','errors','failed','completed']);
  const periods=testCase=>(summary.groups||[]).filter(g=>['fault','recovery'].includes(g.kind)&&testCase.checks.some(c=>c.source===g.id));
  const metric=(group,id)=>group.checks.find(c=>c.id===id);
  const p99Limit=state=>state==='normal'?metric(summary.steady||{checks:[]},'p99')?.limit:state==='recovery'?(summary.groups||[]).filter(g=>g.kind==='recovery').flatMap(g=>g.checks).find(c=>c.id==='p99'&&c.limit!=null)?.limit:null;
  const detailChecks=testCase=>testCase.checks.filter(c=>!latencyCheck(c)&&!(periods(testCase).length&&periodMetrics.has(c.id)));
  const number=(value,digits=1)=>value==null||!Number.isFinite(value)?'—':value.toLocaleString(undefined,{maximumFractionDigits:digits});
  const verdictLabel=check=>check.outcome==='not_measured'&&(check.id==='lost'||check.checks?.some(c=>c.id==='lost'&&c.outcome==='not_measured'))?'Reconciliation required':labels[check.outcome];
  const labels={pass:'Pass',fail:'Fail',recorded:'—',not_measured:'Pending verification',not_run:'Not run',in_progress:'In progress',incomplete:'Incomplete'};
</script>

{#snippet caseRow(testCase)}
  <span class="case-name">{testCase.title}</span><span class="muted">{testCase.expected}</span><span>{testCase.observed}{#if testCase.failureReason}<small class="failed">{testCase.failureReason}</small>{/if}</span><span class="outcome" class:passed={testCase.outcome==='pass'} class:failed={testCase.outcome==='fail'}>{verdictLabel(testCase)}</span>
{/snippet}

<div class="case-table">
  <div class="case-row column-head" aria-hidden="true"><span>Test case</span><span>Expected</span><span>Observed</span><span>Verdict</span></div>
  {#each sections as section (section.id)}
    <section aria-labelledby={`case-section-${section.id}`}>
      <h3 id={`case-section-${section.id}`}>{section.title}</h3>
      {#each section.cases as testCase (testCase.id)}
        {#if detailChecks(testCase).length>1||periods(testCase).length}
        <details class="test-case">
          <summary class="case-row">{@render caseRow(testCase)}</summary>
          <div class="detail-table"><table><thead><tr><th>Check</th><th class="numeric">Limit</th><th class="numeric">Observed</th><th>Verdict</th></tr></thead><tbody>
            {#each detailChecks(testCase) as check,index (`${check.source||''}:${check.id}:${index}`)}<tr><td>{check.label}</td><td class="numeric muted">{formatTestCaseLimit(check)}</td><td class="numeric">{formatTestCaseValue(check)}</td><td><span class="outcome" class:passed={check.outcome==='pass'} class:failed={check.outcome==='fail'}>{verdictLabel(check)}</span></td></tr>{:else}<tr><td colspan="4" class="muted">No execution evidence recorded for this test case.</td></tr>{/each}
          </tbody></table></div>
          {#if periods(testCase).length}
            <div class="detail-table"><table aria-label="Outage and recovery measurements"><thead><tr><th>Period</th><th class="numeric">Successful TPS</th><th class="numeric">HTTP errors</th><th class="numeric">Completed flows</th></tr></thead><tbody>
              {#each periods(testCase) as period}<tr><td>{period.kind==='fault'?'Outage':'After recovery'}</td><td class="numeric">{number(metric(period,'tps')?.actual)}</td><td class="numeric">{number(metric(period,'errors')?.actual)}%</td><td class="numeric">{number(metric(period,'completed')?.actual,0)}</td></tr>{/each}
            </tbody></table></div>
          {/if}
        </details>
        {:else}
          <div class="test-case case-row">{@render caseRow(testCase)}</div>
        {/if}
      {/each}
    </section>
  {/each}
</div>
{#if states.length}
  <h3 class="latency-title">Server-side latency</h3>
  {#if summary.latencySettlingSeconds!=null}<p class="latency-note">Successful payment confirmations in settled periods · {summary.latencySettlingSeconds}s transition settling · restoration and cross-state flows excluded</p>{/if}
  <div class="detail-table latency"><table><thead><tr><th>State</th><th class="numeric">p50 · ms</th><th class="numeric">p95 · ms</th><th class="numeric">p99 · ms</th></tr></thead><tbody>
    {#each states as state}<tr><td>{state.state==='normal'?'Steady load':state.state==='outage'?'Outage':'After recovery'}</td><td class="numeric">{number(state.p50)}</td><td class="numeric">{number(state.p95)}</td><td class="numeric" class:failed={p99Limit(state.state)!=null&&state.p99>p99Limit(state.state)}>{number(state.p99)}{#if p99Limit(state.state)!=null&&state.p99>p99Limit(state.state)}<small>Limit ≤ {number(p99Limit(state.state))} ms</small>{/if}</td></tr>{/each}
  </tbody></table></div>
{/if}

<style>


  .latency-note{margin:-4px 0 10px;color:var(--muted);font-size:11px}
  summary::before{display:none;content:none}summary{font-weight:400;line-height:1.45}
  .case-table{min-width:0;border-top:1px solid var(--border)}.case-row{display:grid;grid-template-columns:minmax(0,1.25fr) minmax(0,1fr) minmax(0,1.4fr) minmax(0,.65fr);gap:16px;align-items:start;min-width:0;font-size:12px;padding:12px 8px}.case-row>span{min-width:0;overflow-wrap:anywhere}.column-head{background:var(--surface-muted);color:var(--muted);font-size:11px;font-weight:500}.case-table h3{margin:0;padding:14px 8px 8px;font-size:12px;font-weight:600}.test-case{border-bottom:1px solid var(--border)}summary{cursor:pointer;list-style:none}summary::-webkit-details-marker{display:none}.case-name{position:relative;padding-left:14px}summary .case-name::before{content:'›';position:absolute;left:0;transition:transform .1s}.test-case[open] summary .case-name::before{transform:rotate(90deg)}.detail-table{min-width:0}.test-case .detail-table{padding:0 8px 12px 22px}.latency-title{margin:20px 0 10px;font-size:12px;font-weight:600}.numeric{text-align:right;white-space:nowrap;font-variant-numeric:tabular-nums}.muted,.outcome{color:var(--muted)}.outcome{font-size:11px;white-space:normal;overflow-wrap:anywhere}.passed{color:var(--success)}.failed{color:var(--danger);font-weight:500}table{width:100%;table-layout:fixed}th,td{padding:9px 12px;font-size:12px;overflow-wrap:anywhere}th:first-child,td:first-child{padding-left:0;width:44%}th:last-child,td:last-child{padding-right:0;padding-left:16px;width:22%}small{display:block;color:var(--muted);font-size:10px;margin-top:3px}@media(max-width:600px){.case-row{grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:8px}.column-head{display:none}.case-name{grid-column:1/-1}.numeric{white-space:normal}th,td{padding:8px 4px}.test-case .detail-table{padding-left:8px}th:first-child,td:first-child{width:38%}th:last-child,td:last-child{padding-left:6px}}
</style>
