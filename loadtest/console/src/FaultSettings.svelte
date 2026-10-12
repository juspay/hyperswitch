<script>
import Icon from './Icon.svelte';
import {onMount,tick} from 'svelte';
import {fade,fly} from 'svelte/transition';
import {faultTypes,faultType,faultEnvironment,configureFaultTarget,switchFaultEnvironment,faultCommandPreview} from './fault-types.mjs';
import {api} from './api.js';
export let targets=[];export let baseline='';
let environment='local',pending=false,message='',selected=null,seconds=5,confirm='',testError='',testDialog,testJob=null,fault=null,checks={},checking={},now=Date.now(),testTargetId='',connected=true,checkTimers={};
$: dirty=JSON.stringify(targets)!==baseline;
$: if(targets.length)environment=targets.every(t=>faultEnvironment(t)===faultEnvironment(targets[0]))?faultEnvironment(targets[0]):'mixed';
$: testing=Boolean(fault?.active||fault?.blocked||fault?.recovering||testJob&&['starting','running','cancelling'].includes(testJob.status));
$: remaining=fault?.active?.recoveryAt?Math.max(0,Math.ceil((fault.active.recoveryAt-now)/1000)):null;
onMount(()=>{let disposed=false,busy=false;const refresh=async()=>{if(busy)return;busy=true;try{const status=await api.faults();if(disposed)return;fault=status;const id=testJob?.id||status.active?.jobId||sessionStorage.getItem('last-fault-test');if(id){const job=await api.job(id);if(!disposed&&job.command==='fault-test'){testJob=job;testTargetId=job.progress?.targetId||status.active?.targetId||testTargetId;}}connected=true;}catch(e){if(!disposed)connected=false;}finally{busy=false;now=Date.now();}};refresh();const timer=setInterval(refresh,1000);return()=>{disposed=true;clearInterval(timer);Object.values(checkTimers).forEach(clearTimeout);};});
async function openTest(target){selected=target;seconds=5;confirm='';testError='';if(testTargetId!==target.id&&!testing){testJob=null;sessionStorage.removeItem('last-fault-test');}await tick();testDialog.showModal();}
function closeTest(){testDialog?.close();selected=null;testError='';}
async function startTest(){pending=true;testError='';try{testJob=await api.testFault({targetId:selected.id,action:selected.actions?.[0]||'outage',durationSeconds:seconds,confirm});testTargetId=selected.id;sessionStorage.setItem('last-fault-test',testJob.id);fault=testJob.faults;}catch(e){testError=e.message;}finally{pending=false;}}
async function restoreNow(){pending=true;try{fault=await api.restoreFault();}catch(e){testError=e.message;}finally{pending=false;}}
function clearCheckTimer(id){if(checkTimers[id])clearTimeout(checkTimers[id]);const {[id]:_,...rest}=checkTimers;checkTimers=rest;}
function resetCheck(id){clearCheckTimer(id);checks={...checks,[id]:null};}
function resetAllChecks(){Object.values(checkTimers).forEach(clearTimeout);checkTimers={};checks={};}
function scheduleCheckReset(id){clearCheckTimer(id);checkTimers={...checkTimers,[id]:setTimeout(()=>resetCheck(id),2400)};}
function checkState(id){return checking[id]?'checking':checks[id]?.ok?'success':checks[id]?'failed':'idle';}
function checkLabel(state){return state==='checking'?'Checking…':state==='success'?'Succeeded':state==='failed'?'Failed':'Check target';}
function switchEnvironment(value){if(testing||pending)return;environment=value;targets=targets.map(target=>switchFaultEnvironment(target,value));resetAllChecks();}
function add(type){if(pending||targets.some(t=>faultType(t)===type))return;const spec=faultTypes.find(t=>t.type===type);targets=[...targets,configureFaultTarget({id:type,label:spec.label,serviceType:type,adapter:spec.adapter,port:spec.port,...(type==='card-vault'?{repository:''}:{binary:'process-compose',controllerPort:8090,process:type==='redis'?'r1':'p1'})},environment==='mixed'?'local':environment)];}
async function check(target){const id=target.id,fingerprint=JSON.stringify(target);resetCheck(id);checking={...checking,[id]:true};try{const result=await api.checkFaultTarget(structuredClone(target));if(JSON.stringify(targets.find(t=>t.id===id))===fingerprint){checks={...checks,[id]:{ok:true,message:result.message}};scheduleCheckReset(id);}}catch(e){if(JSON.stringify(targets.find(t=>t.id===id))===fingerprint)checks={...checks,[id]:{ok:false,message:e.message}};}finally{checking={...checking,[id]:false};}}
</script>

<label>Fault environment · applies to all targets<select value={environment} disabled={testing||pending} onchange={e=>switchEnvironment(e.currentTarget.value)}><option value="mixed" disabled>Mixed · see individual targets</option><option value="local">Local</option><option value="cloud">Cloud</option><option value="custom">Custom</option></select></label>
{#each targets as target,index}
<fieldset oninput={()=>resetCheck(target.id)} onchange={()=>resetCheck(target.id)}>
<legend>{faultTypes.find(t=>t.type===faultType(target))?.label || target.label}</legend>
<details class="target-config"><summary>Configuration <span>{faultEnvironment(target)} · {target.adapter}</span></summary><div class="fields" inert={testing}>
<label>Name<input bind:value={target.label}/></label><details class="target-advanced"><summary>Target identity / environment override</summary><label>Target ID<input readonly value={target.id}/></label><label>Environment<select value={faultEnvironment(target)} onchange={e=>{targets[index]=switchFaultEnvironment(target,e.currentTarget.value);targets=[...targets];resetAllChecks();}}><option value="local">Local</option><option value="cloud">Cloud</option><option value="custom">Custom</option></select></label></details>
{#if faultEnvironment(target)==='custom'}<label>Adapter<select bind:value={target.adapter}><option value="docker">Docker</option><option value="kubernetes">Kubernetes</option></select></label>{/if}
{#if faultEnvironment(target)==='local'}<label>Service port<input type="number" min="1" max="65535" bind:value={target.port}/></label>{/if}
{#if target.adapter==='local-vault'}
<label>Vault repository<input placeholder="/absolute/path/hyperswitch-card-vault" bind:value={target.repository}/></label>
{:else if target.adapter==='process-compose'}
<label>Controller port<input type="number" bind:value={target.controllerPort}/></label><label>Process name<input bind:value={target.process}/></label>
{:else if target.adapter.startsWith('gcp-')}
<label>Project<input bind:value={target.project}/></label><label>Region<input bind:value={target.region}/></label>{#if target.adapter==='gcp-alloydb'}<label>Cluster<input bind:value={target.cluster}/></label>{/if}<label>Primary instance<input bind:value={target.instance}/></label>
{:else if target.adapter==='docker'}<label>Container<input bind:value={target.container}/></label>
{:else if target.adapter==='kubernetes'}<label>Context<input placeholder="Master’s current context" bind:value={target.context}/></label><label>Namespace<input bind:value={target.namespace}/></label><label>Deployment<input bind:value={target.deployment}/></label>{#if faultEnvironment(target)==='cloud'&&faultType(target)==='card-vault'}<label>Recovery replicas<input type="number" min="1" max="100" bind:value={target.recoveryReplicas}/></label>{/if}
{/if}
</div>
{#if faultCommandPreview(target)}<details class="commands"><summary>Commands</summary><pre class="command-preview">{faultCommandPreview(target)}</pre></details>{/if}</details>
<div class="actions"><button type="button" class="secondary check-button" class:check-success={checks[target.id]?.ok} class:check-failed={checks[target.id]&& !checks[target.id].ok} disabled={pending||testing||checking[target.id]||checks[target.id]?.ok} title={checks[target.id]?.message || ''} aria-live="polite" onclick={()=>check(target)}>{#key checkState(target.id)}<span class="check-label" in:fly={{y:5,duration:150}} out:fade={{duration:90}}>{checkLabel(checkState(target.id))}</span>{/key}</button><button type="button" class="secondary" disabled={pending||dirty||testing} onclick={()=>openTest(target)}>{target.actions?.[0]==='failover'?'Test failover':'Test interruption'}</button><button type="button" class="icon-button danger" disabled={pending||testing} aria-label={`Remove ${target.label}`} onclick={()=>{clearCheckTimer(target.id);targets=targets.filter((_,i)=>i!==index);}}><Icon name="close"/></button></div>
{#if checks[target.id]&&!checks[target.id].ok}<p class="invalid" role="alert">{checks[target.id].message}</p>{/if}
{#if dirty}<small>Save scenario to enable interruption testing.</small>{/if}
{#if testJob&&testTargetId===target.id}<button type="button" class="quiet test-status" onclick={()=>openTest(target)}>{testJob.status==='complete'?'✓ Recovery verified':`${target.label} · ${fault?.active?.status||testJob.status}`} · View details</button>{/if}
</fieldset>
{/each}
<dialog class="fault-dialog test-dialog" bind:this={testDialog} aria-labelledby="fault-test-title" onclose={()=>selected=null}>
{#if selected}<div>
<div class="section-head"><div><h1 id="fault-test-title">Test {selected.actions?.[0]==='failover'?'failover':'interruption'}</h1><p>{selected.label}</p></div><button type="button" class="icon-button" aria-label="Close test" onclick={closeTest}><Icon name="close"/></button></div>
{#if testJob&&testTargetId===selected.id}
<div class="test-progress" role="status"><strong>{testJob.status==='complete'?'Recovery verified':fault?.active?.status||testJob.status}</strong>{#if remaining!==null&&fault?.active?.status==='active'}<p>{remaining}s until recovery begins</p>{/if}{#if !connected}<p>Reconnecting… The server continues the test.</p>{/if}{#if testJob.error}<p class="invalid">{testJob.error}</p>{/if}<ol>{#each testJob.events||[] as event}<li>{event.message}</li>{/each}</ol>{#if testing}<button type="button" class="secondary" disabled={pending} onclick={restoreNow}>Restore now</button>{:else}<button type="button" class="secondary" onclick={()=>{testJob=null;testTargetId='';sessionStorage.removeItem('last-fault-test');confirm='';}}>Test again</button>{/if}</div>
{:else}<p class="test-explanation">{selected.actions?.[0]==='failover'?'Promotes the managed standby and verifies that the primary becomes ready.':'Stops the service for the selected downtime, restores it, and verifies recovery.'}</p>
{#if selected.actions?.[0]!=='failover'}<label>Downtime · seconds<input type="number" min="1" max="300" bind:value={seconds}/></label>{/if}
<label>Confirm by typing <strong>{selected.actions?.[0]||'outage'} {selected.id}</strong><input autocomplete="off" bind:value={confirm}/></label>

<div class="actions"><button type="button" class="secondary" onclick={closeTest}>Cancel</button><button type="button" onclick={startTest} disabled={pending||testing||confirm!==`${selected.actions?.[0]||'outage'} ${selected.id}`||!Number.isInteger(seconds)||seconds<1||seconds>300}>{pending?'Starting…':'Start test'}</button></div>{/if}
{#if testError}<div class="message error" role="alert">{testError}</div>{/if}</div>{/if}
</dialog>
{#if message}<p role="status">{message}</p>{/if}
<div class="actions">{#each faultTypes as type}<button type="button" class="secondary" disabled={pending||testing||targets.some(t=>faultType(t)===type.type)} onclick={()=>add(type.type)}>+ {type.label}</button>{/each}</div>

<style>
.target-config>summary{font-size:12px;justify-content:flex-start}.target-config>summary>span{font-size:11px;color:var(--muted);font-weight:400;margin-left:auto}.target-config[open]>.fields{margin-top:20px}.target-advanced{grid-column:1/-1;font-size:12px}.target-advanced[open]>summary{margin-bottom:12px}.target-advanced label{margin:12px 0}.commands{margin-top:18px}.commands summary{font-size:11px}.test-status{display:block;margin-top:8px;font-size:11px}.test-progress ol{padding-left:20px;max-height:240px;overflow:auto;font-size:12px}.test-progress li{padding:5px 0;color:var(--muted)}.test-progress strong{font-size:17px}.command-preview{white-space:pre-wrap;overflow-wrap:anywhere;background:var(--surface-muted);padding:12px;border-radius:6px;font-size:11px}.test-dialog .section-head{margin-bottom:16px}.test-dialog .test-explanation{padding:12px;background:var(--surface-muted);border-radius:8px}.test-dialog strong{display:block;margin-top:4px;color:var(--ink)}.test-dialog .message{margin-top:16px}fieldset{border:1px solid var(--border);border-radius:8px;padding:16px 18px;margin:20px 0;min-width:0}legend{font-weight:550;font-size:13px;padding:0 8px;color:var(--ink)}fieldset>.actions{margin-top:14px}fieldset>small{display:block;font-size:11px;margin-top:8px;text-align:right}.check-button{min-width:112px;overflow:hidden;transition:background-color .18s ease,color .18s ease,border-color .18s ease,box-shadow .18s ease}.check-label{display:inline-flex;align-items:center;justify-content:center}.check-button.check-success:disabled{background:var(--success-soft);border-color:#b5d9c4;color:var(--success);cursor:default;opacity:1;box-shadow:0 0 0 2px color-mix(in srgb,var(--success) 10%,transparent)}
@media(max-width:600px){fieldset{padding:14px}.target-config>summary{flex-wrap:wrap}.target-config>summary>span{margin-left:0}.actions{justify-content:flex-start}.target-config[open]>.fields{gap:14px}fieldset>small{text-align:left}}
</style>
