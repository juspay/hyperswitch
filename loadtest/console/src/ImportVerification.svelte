<script>
 import Icon from './Icon.svelte';
 import {importView} from './import-state.mjs';
 /** @type {any} */ export let job=null;export let pending=false;export let error='';
 let checkPage=0,lastJobId='';
 $: view=importView(job);
 $: if(job?.id!==lastJobId){lastJobId=job?.id||'';checkPage=0;}
</script>
{#if error}<p class="inline-error" role="alert">{error}</p>
{:else if pending&&!view}<p class="verification-message" role="status" aria-live="polite"><span class="spinner"></span>Verifying…</p>
{:else if view}
 <div class="inline-verification" class:passed={view.phase==='ready'} class:failed={view.phase==='failed'}>
  <div class="verification-message" role={view.phase==='failed'?'alert':'status'} aria-live="polite">
   {#if view.busy}<span class="spinner"></span>{:else}<Icon name={view.phase==='ready'?'check':view.phase==='failed'?'close':'info'} size={16}/>{/if}
   <div><strong>{view.title}</strong>{#if view.detail}<p>{view.detail}</p>{/if}</div>
  </div>
  <ol class="verification-stages" aria-label="Import verification stages">
   {#each view.stages as stage,index}<li class:passed={stage.status==='passed'} class:failed={stage.status==='failed'} class:running={stage.status==='running'} aria-current={stage.status==='running'?'step':undefined}>
    <span class="stage-marker">{#if stage.status==='passed'}<Icon name="check" size={12}/>{:else}{index+1}{/if}</span>
    <span>{stage.label}{#if stage.status==='running'&&stage.total!==null}<small>{stage.completed} / {stage.total}</small>{:else if stage.status==='blocked'}<small>Not run</small>{/if}</span>
   </li>{/each}
  </ol>
  {#if view.checks.length}<details class="verification-details"><summary>Details · {job?.progress?.verification.checked??view.checks.length} checks{#if view.failures.length} · {view.failures.length} failed{/if}</summary>
   <div class="table-wrap"><table><thead><tr><th>Merchant</th><th>Check</th><th>Result</th></tr></thead><tbody>{#each view.checks.slice(checkPage*100,(checkPage+1)*100) as check}<tr><td>{check.merchant}</td><td>{check.label}</td><td class:passed={check.ok} class:failed={!check.ok}>{check.ok?'Verified':check.message}</td></tr>{/each}</tbody></table></div>
   {#if view.checks.length>100}<div class="check-pages"><button type="button" class="quiet" disabled={checkPage===0} onclick={()=>checkPage--}>Previous</button><span>{checkPage*100+1}–{Math.min((checkPage+1)*100,view.checks.length)} / {view.checks.length}</span><button type="button" class="quiet" disabled={(checkPage+1)*100>=view.checks.length} onclick={()=>checkPage++}>Next</button></div>{/if}
  </details>{/if}
 </div>
{/if}
<style>
.inline-verification{min-width:0;color:var(--ink)}.verification-message{display:flex;align-items:flex-start;gap:8px;margin:0;font-size:12px;line-height:1.6}.verification-message strong{font-weight:550}.verification-message p{margin:4px 0 0;overflow-wrap:anywhere}.verification-message :global(svg){flex-shrink:0;margin-top:2px}.passed{color:var(--success)}.failed,.inline-error{color:var(--danger)}.inline-error{font-size:12px;line-height:1.6;margin:0;overflow-wrap:anywhere}.verification-stages{display:flex;flex-wrap:wrap;list-style:none;gap:16px 24px;margin:16px 0 0;padding:0}.verification-stages li{display:flex;align-items:center;gap:7px;font-size:12px;color:var(--muted)}.verification-stages li.passed{color:var(--success)}.verification-stages li.failed{color:var(--danger)}.verification-stages li.running{color:var(--accent)}.stage-marker{display:inline-flex;align-items:center;justify-content:center;width:20px;height:20px;border:1px solid var(--border);border-radius:50%;font-size:10px;flex-shrink:0}.running .stage-marker{border-color:var(--accent)}.passed .stage-marker{border-color:var(--success)}small{display:block;margin-top:3px;font-size:10px}.spinner{width:14px;height:14px;margin-top:3px;flex-shrink:0;border:2px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:verify-spin .8s linear infinite}.verification-details{margin-top:16px;color:var(--muted)}summary{font-size:11px}.table-wrap{margin-top:12px}th,td{font-size:11px;padding:10px}td{overflow-wrap:anywhere}.check-pages{display:flex;justify-content:flex-end;align-items:center;gap:12px;margin-top:8px;font-size:11px}@keyframes verify-spin{to{transform:rotate(360deg)}}@media(prefers-reduced-motion:reduce){.spinner{animation:none}}
</style>
