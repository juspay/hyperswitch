<script>
/** @type {any} */
export let result=null;
/** @type {any} */
export let job=null;
export let running=false;
$: failed=job?.status==='failed';
$: current=running||failed?job?.progress||{}:result||job?.progress||{};
$: points=current.timeline||[];
$: compared=current.total??current.compared??0;
$: expected=current.expected??compared;
$: percent=expected?Math.min(100,compared/expected*100):0;
$: end=Math.max(.001,points.at(-1)?.t||current.elapsedSeconds||1);
$: maximum=Math.max(1,compared);
const durationLabel=t=>t<1?`${Math.max(1,Math.round(t*1000))}ms`:`${t.toFixed(1)}s`;
const colors={match:'#398453',missing:'#c43b36',mismatch:'#bd7c21'};
const labels={match:'Matched',missing:'Not found',mismatch:'Status mismatch'};
const x=t=>52+t/end*890;
const y=n=>200-n/maximum*160;
const line=key=>points.map((p,i)=>`${i?'L':'M'}${x(p.t)},${y(p[key]||0)}`).join(' ');
</script>
<section class="recon-summary" aria-label="Reconciliation progress">
<div class="summary-head"><strong>{running?'Comparing payments…':job?.status==='failed'?'Comparison failed':result?'Comparison complete':'Reconciliation'}</strong><span role="status">{compared.toLocaleString()} / {expected.toLocaleString()} compared{#if current.elapsedSeconds!=null} · {durationLabel(current.elapsedSeconds)}{/if}</span></div>
{#if failed&&job?.error}<p class="failure" role="alert">{job.error}</p>{/if}
{#if running}<progress aria-label="Payments compared" max="100" value={percent}></progress>{/if}
{#if points.length}
<svg viewBox="0 0 980 240" role="img" aria-label={`Cumulative reconciliation outcomes over ${end.toFixed(1)} seconds: ${current.match||0} matched, ${current.missing||0} not found, ${current.mismatch||0} status mismatches`}>
{#each [0,.5,1] as fraction}<line x1="52" x2="942" y1={y(maximum*fraction)} y2={y(maximum*fraction)} stroke="#e9edf3"/><text x="43" y={y(maximum*fraction)+4} text-anchor="end">{Math.round(maximum*fraction)}</text>{/each}
{#each Object.keys(labels) as key}<path d={line(key)} fill="none" stroke={colors[key]} stroke-width="2.5"/>{/each}
<text x="52" y="225">0s</text><text x="942" y="225" text-anchor="end">{durationLabel(end)}</text>
</svg>
<p class="chart-caption">Matched / not found / status mismatch · {points.length<=2?'single batch':'comparison progress'}</p>
{:else}<p class="chart-caption">{running?'Connecting to PostgreSQL…':result?'This earlier comparison has no recorded timeline. Compare again to see progress.':'Start a comparison to see progress.'}</p>{/if}
<div class="outcomes">{#each Object.entries(labels) as [key,label]}<div style={`--outcome:${colors[key]}`}><span>{label}</span><strong>{(current[key]||0).toLocaleString()}</strong><small>{compared?((current[key]||0)/compared*100).toFixed(1):'0'}%</small></div>{/each}</div>
{#if result&&!failed}<p class="verdict">{result.missing||result.mismatch?`${result.missing+result.mismatch} discrepancies found.`:result.total===0?'No payment statuses available to compare.':'All compared payments match PostgreSQL.'} {#if result.comparedAt}Compared {new Date(result.comparedAt).toLocaleString()}.{/if}</p>{/if}
</section>
<style>
.failure{color:#c43b36;font-size:13px}.recon-summary{margin-top:20px}.summary-head{display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;font-size:13px}.summary-head span{color:var(--muted,#68707c)}progress{width:100%;height:7px;margin-top:16px;accent-color:#0561e2}.outcomes{display:grid;grid-template-columns:repeat(3,1fr);gap:16px;margin:16px 0}.outcomes>div{border-left:3px solid var(--outcome);padding-left:12px}.outcomes span,.outcomes small{display:block;color:var(--muted,#68707c);font-size:12px}.outcomes strong{display:block;font-size:24px;color:var(--outcome);margin:5px 0}svg{display:block;width:100%;min-height:220px;margin-top:16px}text{font:11px inherit;fill:#8793a5}.chart-caption,.verdict{font-size:12px;color:var(--muted,#68707c)}.verdict{padding-top:14px;border-top:1px solid var(--border,#dce1e8)}@media(max-width:480px){.outcomes{gap:8px}.outcomes strong{font-size:23px}.outcomes span{min-height:30px}}
</style>
