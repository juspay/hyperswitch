<script>
 import InfoTooltip from './InfoTooltip.svelte';
 import ImportVerification from './ImportVerification.svelte';
 import {importView} from './import-state.mjs';
 import {api} from './api.js';import {connectorSet} from './connector-config.mjs';
 export let profile;export let disabled=false;
 /** @type {any} */
 export let job=null;export let onstart=(_job)=>{};export let onreset=()=>{};export let onbusy=(_value)=>{};export let oncancel=()=>{};
 let format='form',merchantId='',profileId='',apiKey='',publishableKey='',accounts={},json='',pending=false,error='',clearedJob='';
 const merchantExample=JSON.stringify([{merchant_id:'merchant_123',profile_id:'pro_123',api_key:'...',publishable_key:'...',connector_accounts:{stripe:'mca_123'}}],null,2);
 $: names=connectorSet(profile?.connectors);
 $: view=importView(job);
 $: collapsed=pending||view?.busy||view?.phase==='ready';
 $: if(view?.phase==='ready'&&clearedJob!==job?.id){clearedJob=job?.id;apiKey='';publishableKey='';json='';accounts={};}
 function edited(){if(pending||view?.busy)return;error='';onreset();}
 function reset(){error='';onreset();}
 async function loadFile(event){edited();const file=event.currentTarget.files?.[0];if(!file)return;if(file.size>900000){error='Choose a JSON file smaller than 900 KB';return;}json=await file.text();}
 async function submit(){if(disabled||pending||view?.busy)return;pending=true;onbusy(true);error='';onreset();try{const merchants=format==='json'||profile?.mode==='tenancy'?JSON.parse(json):[{merchant_id:merchantId,profile_id:profileId,api_key:apiKey,publishable_key:publishableKey,connector_accounts:accounts}];const value=await api.importFixtures({profileId:profile.id,merchants});onstart(value);}catch(e){error=e.message;}finally{pending=false;onbusy(false);}}
</script>
<div class="import-form"><p>Import existing fixtures <InfoTooltip id="fixture-import-info" label="Fixture import information" text="Use existing merchants and connector accounts without changing their remote configuration. Verification checks credentials, ownership, and the connector setup required by this scenario. Configure the Router admin key in Connection settings for publishable-key inspection. Use a separate smoke test to check payment execution."/></p>
{#if !collapsed}<div class="import-inputs" oninput={edited} onchange={edited}>{#if profile?.mode!=='tenancy'}<label>Import format<select bind:value={format}><option value="form">Single merchant</option><option value="json">JSON</option></select></label>{/if}
{#if format==='json'||profile?.mode==='tenancy'}<label>Merchant JSON file<input type="file" accept=".json,application/json" onchange={loadFile} disabled={disabled||pending}/></label><div class="import-field"><div class="field-title"><label for="import-merchant-json">Merchant JSON</label><InfoTooltip id="merchant-json-info" label="Merchant JSON information" text={`Import exactly ${profile?.merchantCount} merchant${profile?.merchantCount===1?'':'s'} as a JSON array, including credentials and the connector-to-MCA-ID mapping.`}/></div><textarea id="import-merchant-json" rows="12" bind:value={json} placeholder={merchantExample} autocomplete="off" spellcheck="false"></textarea></div>
{:else}<div class="fields"><label>Merchant ID<input bind:value={merchantId} autocomplete="off"/></label><label>Profile ID<input bind:value={profileId} autocomplete="off"/></label><label>API key<input type="password" bind:value={apiKey} autocomplete="new-password"/></label><div class="import-field"><div class="field-title"><label for="import-publishable-key">Publishable key</label><InfoTooltip id="publishable-key-info" label="Publishable key information" text="Required for SDK and modular payment-method calls."/></div><input id="import-publishable-key" type="password" bind:value={publishableKey} autocomplete="new-password"/></div>{#each names as name}<label>{name} MCA ID<input bind:value={accounts[name]} autocomplete="off"/></label>{/each}</div>{/if}
</div>{:else}<p class="import-summary">{profile?.merchantCount} {profile?.merchantCount===1?'merchant':'merchants'} · {names.join(', ')}</p>{/if}
<ImportVerification {job} {pending} {error}/>
<div class="actions">{#if view?.busy}<button type="button" class="secondary" disabled={job?.status==='cancelling'} onclick={oncancel}>{job?.status==='cancelling'?'Cancelling…':'Cancel'}</button>{:else if view?.phase==='ready'}<button type="button" class="quiet" onclick={reset}>Import another</button><a class="button" href={`/performance?profile=${profile.id}&fixtures=${view.fixtureSetId}`}>Use fixture →</a>{:else}<button type="button" disabled={disabled||pending||!profile} onclick={submit}>{pending?'Verifying…':view?'Try again':'Import and verify'}</button>{/if}</div></div>
<style>.import-inputs{display:grid;gap:16px}.import-summary{color:var(--muted)}.import-field{display:grid;gap:8px;align-content:start}.field-title{display:flex;align-items:center}.field-title label{margin:0}.import-form{grid-column:1/-1;border-top:1px solid var(--border);padding-top:16px;display:grid;gap:16px}.import-form p{font-size:12px;margin:0}textarea{width:100%;font-family:monospace;line-height:1.6;tab-size:2}.actions{justify-content:flex-end}</style>
