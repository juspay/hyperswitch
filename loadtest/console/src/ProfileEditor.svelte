<script>
import {onMount} from 'svelte';import {beforeNavigate,goto} from '$app/navigation';
import FaultSettings from './FaultSettings.svelte';
import {api} from './api.js';import RampEditor from './RampEditor.svelte';import FlowMix from './FlowMix.svelte';import {profileSummary} from './flow-catalog.mjs';import {loadShape} from './load-shape.mjs';
export let profile;export let creating=false;
let draft=structuredClone(profile),baseline,pending=false,error='',notice='',rampValid=true,targets=[];
async function refreshTargets(){try{targets=(await api.faults()).targets;}catch(e){error=e.message;}}
if(creating){draft.name='New scenario';delete draft.id;delete draft.version;}baseline=creating?'':JSON.stringify(draft);
$: dirty=JSON.stringify(draft)!==baseline;
$: summary=(()=>{if(draft.loadShape!=='staged')return profileSummary(draft);try{const shape=loadShape({start_rps:Number(draft.startRps),stages:JSON.parse(draft.stagesJson)});return profileSummary({...draft,totalRps:shape.peak,durationSeconds:shape.duration});}catch{return 'Configure valid load stages';}})();
$: allocated=draft.scenarios.reduce((sum,flow)=>sum+Number(flow.weight||0),0);
$: canSave=draft.name.trim().length>0&&allocated===100&&draft.scenarios.every(flow=>Number.isFinite(flow.weight)&&flow.weight>=0&&flow.weight<=100)&& (draft.loadShape!=='staged'||rampValid);
beforeNavigate(({cancel})=>{if(pending){cancel();return;}if(dirty&&!window.confirm('Discard unsaved scenario changes?'))cancel();});
onMount(()=>{refreshTargets();window.addEventListener('console-connections-saved',refreshTargets);const unload=e=>{if(dirty){e.preventDefault();e.returnValue='';}};window.addEventListener('beforeunload',unload);return()=>{window.removeEventListener('beforeunload',unload);window.removeEventListener('console-connections-saved',refreshTargets);};});
async function save(){pending=true;error='';notice='';try{draft=await api.saveProfile(draft);baseline=JSON.stringify(draft);notice='Scenario saved';return true;}catch(e){error=e.message;return false;}finally{pending=false;}}
async function submit(){if(await save())await goto(`/scenarios?tab=scenarios&view=edit&profile=${encodeURIComponent(draft.id)}`,{replaceState:true,keepFocus:true,noScroll:true});}
async function performance(){if(dirty&&!await save())return;await goto(`/performance?profile=${encodeURIComponent(draft.id)}`);}
</script>
<div class="section-head"><div><a class="quiet" href="/scenarios">← Back to scenarios</a><h1>{creating?'New scenario':draft.name}</h1><p>{summary}</p></div></div>
{#if error}<div class="message error" role="alert">{error}</div>{/if}
<form onsubmit={e=>{e.preventDefault();submit();}}>
<fieldset disabled={pending}>
<div class="basics"><label>Scenario name<input required maxlength="100" bind:value={draft.name}/></label><label>Merchant mode<select bind:value={draft.mode} onchange={()=>draft.merchantCount=draft.mode==='single'?1:1500}><option value="single">Single · 1 merchant</option><option value="tenancy">Tenancy · 1,500 merchants</option></select></label>{#if draft.loadShape==='flat'}<label>Offered TPS<input required type="number" min="1" bind:value={draft.totalRps}/></label><label>Duration · seconds<input required type="number" min="1" bind:value={draft.durationSeconds}/></label>{/if}</div>
<div class="shape"><label>Load shape<select bind:value={draft.loadShape}><option value="flat">Constant TPS</option><option value="staged">Timeline · Hold, Ramp, Fault</option></select></label></div>
{#if draft.loadShape==='staged'}<RampEditor {targets} bind:startRps={draft.startRps} bind:stagesJson={draft.stagesJson} bind:valid={rampValid}/>{/if}
<FlowMix bind:flows={draft.scenarios} disabled={pending}/>
<details class="advanced"><summary>Advanced</summary><div class="fields">{#if draft.mode==='tenancy'}<label>Merchant count<input required type="number" min="2" max="1500" bind:value={draft.merchantCount}/></label>{/if}<label>Preallocated VUs / TPS<input required type="number" min="0.1" step="0.1" bind:value={draft.preAllocatedVusMultiplier}/></label><label>Maximum VUs / TPS<input required type="number" min="0.1" step="0.1" bind:value={draft.maxVusMultiplier}/></label><label>Minimum success ratio<input required type="number" min="0" max="1" step="0.01" bind:value={draft.acceptance.minimumSuccessRatio}/></label><label>Maximum HTTP errors · %<input required type="number" min="0" max="100" bind:value={draft.acceptance.maximumErrorPercent}/></label><label>Maximum confirm p99 · ms<input required type="number" min="0" bind:value={draft.acceptance.maximumConfirmP99Ms}/></label><label>Maximum dropped flows<input required type="number" min="0" bind:value={draft.acceptance.maximumDropped}/></label></div></details>
</fieldset>
<details class="advanced"><summary>Fault targets</summary><FaultSettings/></details>
<div class="flow-footer"><span class="save-status" role="status">{pending?'Saving…':dirty?'Unsaved changes':notice || `Saved · version ${draft.version}`}</span><div class="actions"><button class="secondary" type="submit" disabled={pending||!dirty||!canSave}>{pending?'Saving…':'Save scenario'}</button><button type="button" disabled={pending||!canSave} onclick={performance}>Open Performance →</button></div></div>
</form>
<style>fieldset{border:0;padding:0;margin:0;min-width:0}.section-head h1{margin-top:14px}.basics{display:grid;grid-template-columns:1.4fr 1fr .7fr .7fr;gap:18px;background:white;border:1px solid var(--border);border-radius:12px;padding:22px}.basics label{font-weight:600}.shape{margin:18px 0}.shape label{max-width:230px}.save-status{font-size:12px;color:#667085}.advanced{background:#fff;border:1px solid var(--border);border-radius:12px;padding:16px}.advanced summary{margin-bottom:0}.advanced[open] summary{margin-bottom:16px}@media(max-width:850px){.basics{grid-template-columns:1fr 1fr}}@media(max-width:480px){.basics{grid-template-columns:1fr;padding:16px}}
</style>
