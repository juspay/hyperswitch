<script>
import {tick} from 'svelte';
import {flowCatalog,flowLabel,newFlow} from './flow-catalog.mjs';
export let flows;export let disabled=false;
let expanded=null,picker,search='',searchInput;
$: allocated=flows.reduce((sum,flow)=>sum+Number(flow.weight||0),0);
$: choices=flowCatalog.filter(([,label])=>label.toLowerCase().includes(search.trim().toLowerCase()));
async function openPicker(){search='';picker.showModal();await tick();searchInput.focus();}
function add(type){flows=[...flows,newFlow(type,flows)];picker.close();expanded=flows.length-1;}
function remove(index){flows=flows.filter((_,i)=>i!==index);expanded=null;}
</script>
<section class="mix" aria-labelledby="mix-title">
<div class="section-head"><div><h2 id="mix-title">Flow mix</h2><p class:invalid={allocated!==100}>{allocated===100?'100% allocated':allocated<100?`${100-allocated}% remaining`:`${allocated-100}% over allocated`}</p></div><button type="button" class="secondary" disabled={disabled} onclick={openPicker}>+ Add flow</button></div>
<div class="flow-list">
{#each flows as flow,index}
<div class="flow-row">
<button type="button" class="flow-name" disabled={disabled} aria-expanded={expanded===index} aria-controls={`flow-options-${index}`} onclick={()=>expanded=expanded===index?null:index}>{flowLabel(flow.scenario)}<span class="integration">{flow.merchant_path==='modular'?'Modular':'Non-modular'}</span><span aria-hidden="true" class="chevron">{expanded===index?'⌃':'⌄'}</span></button>
<label class="flow-weight"><input aria-label={`${flowLabel(flow.scenario)} weight`} type="number" min="0" max="100" required disabled={disabled} bind:value={flow.weight}/><span>%</span></label>
<button type="button" class="icon-button" disabled={disabled} aria-label={`Remove ${flowLabel(flow.scenario)}`} onclick={()=>remove(index)}>×</button>
</div>
{#if expanded===index}
<div class="flow-options fields" id={`flow-options-${index}`}>
<label>Flow<select disabled={disabled} bind:value={flow.scenario}>{#each flowCatalog as [type,label]}<option value={type}>{label}</option>{/each}</select></label>
<label>Integration<select disabled={disabled} bind:value={flow.merchant_path}><option value="modular">Modular</option><option value="non_modular">Non-modular</option></select></label>
<label class="internal-name">Internal name <span>Used in run metrics</span><input disabled={disabled} required bind:value={flow.name}/></label>
</div>
{/if}
{/each}
{#if !flows.length}<p class="muted">Add a flow to configure the traffic mix.</p>{/if}
</div>
</section>
<dialog bind:this={picker} aria-labelledby="picker-title"><div class="section-head"><h2 id="picker-title">Add flow</h2><button type="button" class="icon-button" aria-label="Close flow picker" onclick={()=>picker.close()}>×</button></div><input type="search" aria-label="Search flows" placeholder="Search flows" bind:this={searchInput} bind:value={search}/><div class="choices">{#each choices as [type,label]}<button type="button" class="choice" onclick={()=>add(type)}>{label}<span aria-hidden="true">+</span></button>{/each}{#if !choices.length}<p>No flows found.</p>{/if}</div></dialog>
<style>
.mix{background:white;border:1px solid var(--border);border-radius:12px;padding:22px;margin:24px 0}.mix .section-head{align-items:center;margin-bottom:12px}.mix h2{font-size:16px;margin:0}.mix p{font-size:12px}.flow-row{display:grid;grid-template-columns:minmax(0,1fr) 92px 32px;gap:16px;align-items:center;border-top:1px solid #edf0f4;padding:12px 0}.flow-name{display:flex;align-items:center;gap:14px;text-align:left;background:transparent;color:#1d2939;padding:8px 0;font-weight:500}.flow-name:hover{color:var(--accent);background:transparent}.integration{font-size:12px;color:#667085;font-weight:400}.chevron{margin-left:auto;color:#98a2b3}.flow-weight{display:flex;align-items:center;gap:7px}.flow-weight input{margin:0;text-align:center;width:72px;appearance:textfield;-moz-appearance:textfield}.flow-weight input::-webkit-inner-spin-button{appearance:none}.flow-options{padding:8px 0 20px}.internal-name span{font-weight:400;color:#98a2b3}.internal-name{grid-column:1/-1}.internal-name input{max-width:420px}dialog{width:min(440px,calc(100vw - 32px));padding:24px;border:1px solid var(--border);border-radius:16px;box-shadow:0 24px 80px #10182830}dialog::backdrop{background:#10182855}.choices{margin-top:14px;max-height:340px;overflow:auto}.choice{display:flex;align-items:center;justify-content:space-between;width:100%;padding:12px;text-align:left;background:white;color:#344054;font-weight:400}.choice:hover{background:#f2f4f7;color:var(--accent)}@media(max-width:600px){.mix{padding:16px}.flow-row{gap:8px;grid-template-columns:minmax(0,1fr) 82px 32px}.flow-name{flex-wrap:wrap;gap:3px}.integration{width:100%;order:2}.chevron{display:none}}
</style>
