<script>
/** @type {any} */ export let result=null;
/** @type {any} */ export let job=null;
export let running=false;
$: failed=job?.status==='failed';
$: current=running||failed?job?.progress||{}:result||{};
$: compared=current.compared??current.total??0;
$: expected=current.expected??compared;
$: percent=expected?Math.min(100,compared/expected*100):0;
const durationLabel=t=>t<1?`${Math.max(1,Math.round(t*1000))}ms`:`${t.toFixed(1)}s`;
</script>
<section class="recon-summary" aria-label="Reconciliation progress">
{#if running||result||failed}<div class="summary-head"><strong class:failure={failed}>{running?'Comparing…':failed?'Comparison failed':'Reconciled'}</strong><span role="status">{#if running}{expected?`${compared.toLocaleString()} / ${expected.toLocaleString()} payment IDs compared`:'Preparing comparison…'}{:else if result}{(result.accounted??compared).toLocaleString()} outcomes{/if}{#if current.elapsedSeconds!=null} · {durationLabel(current.elapsedSeconds)}{/if}</span></div>{/if}
{#if failed&&job?.error}<p class="failure" role="alert">{job.error}</p>{/if}
{#if running}<progress aria-label="Payments compared" max="100" value={percent}></progress>{/if}
<slot><p class="chart-caption">Reconcile recorded outcomes with the configured database.</p></slot>
</section>
<style>.failure{color:#c43b36;font-size:13px}.summary-head{display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;font-size:11px;margin-bottom:16px}.summary-head span{color:var(--muted);font-variant-numeric:tabular-nums}.summary-head strong{font-weight:500;color:var(--success)}.summary-head strong.failure{color:var(--danger)}progress{width:100%;height:7px;margin-bottom:16px;accent-color:#0561e2}.chart-caption{font-size:12px;color:var(--muted)}</style>
