<script>
import {connectorCatalog,defaultConnectorPlan,connectorPlanError,supportedConnectorAuth} from './connector-config.mjs';
export let value=defaultConnectorPlan();
let choice='';
$: options=connectorCatalog.filter(c=>c.card&&supportedConnectorAuth.has(c.authType)&&c.dummy===(value.mode==='dummy')&&!value.mix.some(r=>r.connector===c.id));
$: error=connectorPlanError(value);
function mode(mode){value=mode==='dummy'?defaultConnectorPlan():{mode,mix:[]};choice='';}
function add(){if(!choice)return;const mix=[...value.mix,{connector:choice,weight:0}];const base=Math.floor(100/mix.length);value={...value,mix:mix.map((r,i)=>({...r,weight:base+(i<100%mix.length?1:0)}))};choice='';}
function remove(index){value={...value,mix:value.mix.filter((_,i)=>i!==index)};}
</script>
<div class="connector-mix">
<label>Connector mode<select value={value.mode} onchange={e=>mode(e.currentTarget.value)}><option value="dummy">Dummy</option><option value="real">Real connectors · sandbox</option></select></label>
{#each value.mix as row,i (row.connector)}<div class="connector-row"><span>{connectorCatalog.find(c=>c.id===row.connector)?.label||row.connector}</span><label>Traffic · %<input type="number" min="1" max="100" step="1" bind:value={row.weight}/></label><button type="button" class="quiet" aria-label={`Remove ${row.connector}`} onclick={()=>remove(i)}>Remove</button></div>{/each}
{#if options.length}<div class="connector-add"><label>Add connector<select bind:value={choice}><option value="">Choose connector</option>{#each options as option}<option value={option.id}>{option.label}</option>{/each}</select></label><button type="button" class="secondary" disabled={!choice||value.mix.length>=20} onclick={add}>Add</button></div>{/if}
{#if error}<p class="invalid" role="status">{error}</p>{/if}
</div>
<style>.connector-mix{max-width:560px;margin-top:14px}.connector-mix>label{max-width:260px}.connector-row{display:grid;grid-template-columns:1fr 100px auto;gap:16px;align-items:center;padding:12px 0;border-bottom:1px solid var(--border)}.connector-add{display:flex;gap:12px;align-items:end;margin-top:16px}.connector-add label{flex:1}.connector-mix p{font-size:12px}</style>
