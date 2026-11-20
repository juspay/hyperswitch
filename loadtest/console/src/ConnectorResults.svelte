<script>
export let result;
/** @type {{scope?:string,successful:number,outageAttempts:number,outageFailures:number}|null} */
export let completion=null;
const number=v=>v==null?'—':Number(v).toLocaleString(undefined,{maximumFractionDigits:1});
const statistics=['min','max','mean','median','p99'];
const latencyPaths=[['hs','HS-only'],['full','Full confirm']];
$: rows=[...(result?.rows||[]),...(result?.inconclusive?[result.inconclusive]:[])];
$: latencyRows=rows.filter(row=>row.connector!==null||row.latency?.hs?.count||row.latency?.full?.count);
$: totalSucceeded=rows.reduce((sum,row)=>sum+row.successes,0);
$: outageSucceeded=completion?completion.outageAttempts-completion.outageFailures:0;
$: reconciled=completion?.scope==='payment_confirm_outside_outage'&&completion.successful+outageSucceeded===totalSucceeded;
</script>
{#if rows.length}
<section class="connector-results">
 <div class="connector-heading"><h2>Connectors</h2><span>All periods</span></div>
 <div class="table-wrap"><table><thead><tr><th>Connector</th><th>Assigned flows</th><th title="Routed share among confirmations that reported a connector">Target / routed</th><th>Confirmations</th><th>Succeeded</th><th>Failed</th></tr></thead><tbody>
 {#each rows as row (row.connector)}<tr><td class:inconclusive={row.connector===null}>{row.connector??'Inconclusive'}</td><td>{number(row.assigned)}</td><td>{row.connector?`${number(row.targetPercent)}% / ${number(row.observedPercent)}%`:'—'}</td><td>{number(row.confirmations)}</td><td>{number(row.successes)}</td><td>{number(row.failures)}</td></tr>{/each}
 </tbody></table></div>
 {#if completion&&reconciled&&completion.outageAttempts}<p>{number(completion.successful)} succeeded outside outage · {number(outageSucceeded)} overlapping outage.</p>{/if}
 {#if result.inconclusive}<p>Inconclusive: connector not reported.{#if result.inconclusive.http5xx}{' '}{number(result.inconclusive.http5xx)} HTTP 5xx {result.inconclusive.http5xx===1?'response':'responses'}.{/if}{' '}Confirmation outcomes remain counted.</p>{/if}
 <div class="latency-heading"><h3>Steady-load latency <span>· ms</span></h3><p>Successful confirmations before faults and after recovery settling.</p></div>
 {#if result.latencyScope==='settled_steady_load'}
 <div class="table-wrap"><table class="latency-statistics" aria-label="Steady-load connector latency"><thead><tr><th>Connector</th><th>Latency</th><th class="numeric">Samples</th><th class="numeric">Min</th><th class="numeric">Max</th><th class="numeric">Mean</th><th class="numeric">Median</th><th class="numeric">p99</th></tr></thead><tbody>
 {#each latencyRows as row (row.connector)}{#each latencyPaths as [key,label],index}
 <tr>{#if index===0}<td rowspan="2">{row.connector??'Inconclusive'}</td>{/if}<td>{label}</td><td class="numeric">{number(row.latency[key].count)}</td>{#each statistics as stat}<td class="numeric" class:p99={stat==='p99'}>{number(row.latency[key][stat])}</td>{/each}</tr>
 {/each}{/each}
 </tbody></table></div>
 {:else}<p>Steady-load latency is unavailable for this recording.</p>{/if}
</section>
{/if}
<style>
.connector-results{margin:24px 0;background:var(--surface);border:1px solid var(--border);border-radius:var(--radius);padding:20px}.connector-heading{display:flex;align-items:center;gap:12px;justify-content:space-between;margin-bottom:16px}.connector-heading span,.connector-results p{font-size:12px;color:var(--muted)}.connector-results p{margin:12px 0 0}.inconclusive{color:var(--muted)}.latency-heading{margin-top:22px;padding-top:18px;border-top:1px solid var(--border)}.latency-heading h3{font-size:13px;margin:0}.latency-heading h3 span{color:var(--muted);font-weight:400}.latency-heading p{margin:5px 0 12px;font-size:11px}.numeric{text-align:right;font-variant-numeric:tabular-nums}.p99{font-weight:550}.latency-statistics td,.latency-statistics th{padding:11px 12px}.latency-statistics td[rowspan]{vertical-align:middle}.latency-statistics{min-width:600px}
</style>
