<script>
import {onMount} from 'svelte';
import {faultTypes,faultType,faultEnvironment,configureFaultTarget,faultCommandPreview} from './fault-types.mjs';
import {api} from './api.js';
let environment='local';
let targets=[],dirty=false,pending=false,message='',selected=null,seconds=5,confirm='',testJob=null,fault=null;
$: testing=Boolean(fault?.active||fault?.blocked||fault?.recovering||testJob&&['starting','running','cancelling'].includes(testJob.status));
onMount(()=>{let disposed=false,busy=false;const refresh=async()=>{if(busy)return;busy=true;try{const status=await api.faults();if(disposed)return;fault=status;if(testJob||status.active?.jobId){const job=await api.job(testJob?.id||status.active.jobId);if(!disposed&&job.command==='fault-test')testJob=job;}}catch(e){if(!disposed)message=e.message;}finally{busy=false;}};api.faultTargets().then(value=>{if(!disposed){targets=value;if(value.length&&value.every(t=>faultEnvironment(t)===faultEnvironment(value[0])))environment=faultEnvironment(value[0]);}}).catch(e=>{if(!disposed)message=e.message;});refresh();const timer=setInterval(refresh,1000);return()=>{disposed=true;clearInterval(timer);};});
async function startTest(){pending=true;try{testJob=await api.testFault({targetId:selected.id,action:selected.actions?.[0]||'outage',durationSeconds:seconds,confirm});fault=testJob.faults;selected=null;message='Fault test started';}catch(e){message=e.message;}finally{pending=false;}}
async function restoreNow(){pending=true;try{fault=await api.restoreFault();message='Recovery requested';}catch(e){message=e.message;}finally{pending=false;}}

function switchEnvironment(value){if(testing||pending)return;environment=value;targets=targets.map(target=>configureFaultTarget(target,value));dirty=targets.length>0;message='';}
function add(type){if(pending||targets.some(t=>faultType(t)===type))return;const spec=faultTypes.find(t=>t.type===type);targets=[...targets,configureFaultTarget({id:type,label:spec.label,serviceType:type,adapter:spec.adapter,port:spec.port,...(type==='card-vault'?{repository:''}:{binary:'process-compose',controllerPort:8090,process:type==='redis'?'r1':'p1'})},environment)];dirty=true;}
async function save(){pending=true;try{targets=await api.saveFaultTargets(targets);dirty=false;message='Fault targets saved';window.dispatchEvent(new Event('console-connections-saved'));}catch(e){message=e.message;}finally{pending=false;}}
async function check(target){pending=true;try{message=`${target.label}: ${(await api.checkFaultTarget(target)).message}`;}catch(e){message=e.message;}finally{pending=false;}}
</script>

<label>Environment<select value={environment} disabled={testing||pending} onchange={e=>switchEnvironment(e.currentTarget.value)}><option value="local">Local</option><option value="cloud">Cloud</option><option value="custom">Custom</option></select></label>
{#each targets as target,index}
<fieldset disabled={testing} oninput={()=>dirty=true} onchange={()=>dirty=true}>
<legend>{faultTypes.find(t=>t.type===faultType(target))?.label || target.label}</legend>
<div class="fields">
<label>Name<input bind:value={target.label}/></label><label>Target ID<input bind:value={target.id}/></label><label>Environment<select value={faultEnvironment(target)} onchange={e=>{targets[index]=configureFaultTarget(target,e.currentTarget.value);targets=[...targets];dirty=true;}}><option value="local">Local</option><option value="cloud">Cloud</option><option value="custom">Custom</option></select></label>
{#if faultEnvironment(target)==='custom'}<label>Adapter<select bind:value={target.adapter}><option value="docker">Docker</option><option value="kubernetes">Kubernetes</option></select></label>{/if}
{#if faultEnvironment(target)==='local'}<label>Service port<input type="number" min="1" max="65535" bind:value={target.port}/></label>{/if}
{#if target.adapter==='local-vault'}
<label>Vault repository<input placeholder="/absolute/path/hyperswitch-card-vault" bind:value={target.repository}/></label>
{:else if target.adapter==='process-compose'}
<label>Process-compose binary<input bind:value={target.binary}/></label><label>Controller port<input type="number" bind:value={target.controllerPort}/></label><label>Process name<input bind:value={target.process}/></label>
{:else if target.adapter.startsWith('gcp-')}
<label>Project<input bind:value={target.project}/></label><label>Region<input bind:value={target.region}/></label>{#if target.adapter==='gcp-alloydb'}<label>Cluster<input bind:value={target.cluster}/></label>{/if}<label>Primary instance<input bind:value={target.instance}/></label>
{:else if target.adapter==='docker'}<label>Container<input bind:value={target.container}/></label>
{:else if target.adapter==='kubernetes'}<label>Context<input placeholder="Worker’s current context" bind:value={target.context}/></label><label>Namespace<input bind:value={target.namespace}/></label><label>Deployment<input bind:value={target.deployment}/></label>
{/if}
</div>
{#if faultCommandPreview(target)}<pre class="command-preview">{faultCommandPreview(target)}</pre>{/if}
<div class="actions"><button type="button" class="secondary" disabled={pending} onclick={()=>check(target)}>Check target</button><button type="button" class="secondary" disabled={pending||dirty||testing} onclick={()=>{selected=target;seconds=5;confirm='';}}>{target.actions?.[0]==='failover'?'Test failover':'Test outage'}</button><button type="button" class="icon-button" aria-label={`Remove ${target.label}`} onclick={()=>{targets=targets.filter((_,i)=>i!==index);dirty=true;}}>×</button></div>
</fieldset>
{/each}
{#if selected}
<section class="test-panel" aria-label="Confirm fault test">
<h3>Test {selected.label} · {selected.actions?.[0]||'outage'}</h3>
<p>{selected.actions?.[0]==='failover'?'Triggers managed standby promotion and verifies readiness.':'Stops the service, waits, then restores it and verifies recovery.'}</p>
<div class="fields">{#if selected.actions?.[0]!=='failover'}<label>Duration · seconds<input type="number" min="1" max="300" bind:value={seconds}/></label>{/if}<label>Type “{selected.actions?.[0]||'outage'} {selected.id}” to confirm<input autocomplete="off" bind:value={confirm}/></label></div>
<div class="actions"><button type="button" class="secondary" onclick={()=>selected=null}>Cancel</button><button type="button" disabled={pending||testing||confirm!==`${selected.actions?.[0]||'outage'} ${selected.id}`||!Number.isInteger(seconds)||seconds<1||seconds>300} onclick={startTest}>Start test</button></div>
</section>
{/if}
{#if testJob||fault?.active||fault?.blocked}
<section class="test-panel" aria-live="polite"><h3>{testJob?'Fault test':'Fault recovery'} · {fault?.active?.status||testJob?.status||'pending'}</h3>
{#if testJob?.error}<p role="alert">{testJob.error}</p>{/if}
{#each testJob?.events||[] as event}<p>{event.message}</p>{/each}
{#if fault?.active||fault?.blocked}<button type="button" class="secondary" disabled={pending} onclick={restoreNow}>Restore now</button>{/if}
</section>
{/if}
{#if message}<p role="status">{message}</p>{/if}
<div class="actions">{#each faultTypes as type}<button type="button" class="secondary" disabled={pending||testing||targets.some(t=>faultType(t)===type.type)} onclick={()=>add(type.type)}>+ {type.label}</button>{/each}<button type="button" disabled={pending||testing||!dirty} onclick={save}>{pending?'Working…':'Save faults'}</button></div>

<style>.command-preview{white-space:pre-wrap;overflow-wrap:anywhere;background:var(--surface-muted,#f5f7fa);padding:12px;border-radius:8px;font-size:12px;}.test-panel{border:1px solid var(--border,#dce1e8);border-radius:12px;padding:16px;margin:16px 0;}.test-panel h3{margin-top:0;}fieldset{border:1px solid var(--border,#dce1e8);border-radius:12px;padding:16px;margin:16px 0;}legend{font-weight:600;padding:0 8px;}</style>
