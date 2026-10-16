<script>
import {onMount} from 'svelte';
import {api} from './api.js';
import {isDatabaseTarget,faultHealthOverride,pendingHealth} from './service-health.mjs';

export let settings;
let services=[],refreshing=false,refreshQueued=false,timer;

const labelFor=target=>target.serviceType==='card-vault'?'Card vault':target.serviceType==='redis'?'Redis':target.label;
// Database reachability is always defined by the saved Settings connection. Older
// fault targets may lack serviceType, so recognize their legacy PostgreSQL labels too.
function configuredServices(targets=[]){
  const result=[
    {id:'router',label:'Router',check:()=>api.testService('router',settings.router)},
    {id:'pm',label:'PM service',check:()=>api.testService('modularPm',settings.modularPm)},
    {id:'database',label:'DB',check:()=>api.testDatabase(settings.database)}
  ];
  if(settings.superposition?.endpoint)result.push({id:'superposition',label:'Superposition',check:()=>api.testService('superposition',settings.superposition.endpoint,settings.superposition)});
  for(const target of targets){
    if(target.adapter==='simulated'||isDatabaseTarget(target))continue;
    const label=labelFor(target);
    if(result.some(service=>service.label===label))continue;
    result.push({id:`fault-${target.id}`,targetId:target.id,label,check:()=>api.checkFaultTarget(target)});
  }
  return result;
}
async function refresh(){
  if(refreshing){refreshQueued=true;return;}
  refreshing=true;
  let targets=[],fault={};
  try{fault=await api.faults();targets=(fault.targets||[]).map(item=>item.target||item);}catch{}
  const configured=configuredServices(targets),previous=new Map(services.map(service=>[service.id,service]));
  services=configured.map(service=>pendingHealth(service,previous.get(service.id),faultHealthOverride(service,fault,targets)));
  await Promise.all(configured.map(async service=>{
    const override=faultHealthOverride(service,fault,targets);if(override)return;
    try{const result=await service.check();services=services.map(item=>item.id===service.id?{...item,state:'online',message:result.message||'Available'}:item);}
    catch(error){services=services.map(item=>item.id===service.id?{...item,state:'offline',message:error.message}:item);}
  }));
  refreshing=false;
  if(refreshQueued){refreshQueued=false;refresh();}
}
onMount(()=>{
  const saved=()=>refresh(),faultChanged=()=>refresh();
  refresh();timer=setInterval(refresh,30000);
  window.addEventListener('console-connections-saved',saved);
  window.addEventListener('console-fault-state',faultChanged);
  return()=>{clearInterval(timer);window.removeEventListener('console-connections-saved',saved);window.removeEventListener('console-fault-state',faultChanged);};
});
</script>

<div class="service-indicators" aria-label="Configured service status" aria-live="polite">
  {#each services as service}
    <button type="button" class="service-indicator" class:online={service.state==='online'} class:offline={service.state==='offline'} class:checking={service.state==='checking'} class:recovering={service.state==='recovering'} onclick={refresh} title={`${service.label}: ${service.message}`} aria-label={`${service.label}: ${service.message}`}>
      <span aria-hidden="true"></span><b>{service.label}</b>
    </button>
  {/each}
</div>

<style>
.service-indicators{display:flex;flex-direction:column;align-items:stretch;gap:8px;width:100%}
.service-indicator{--button-bg:transparent;--button-fg:#617087;--button-border:transparent;--button-hover-bg:var(--hover-bg);--button-hover-fg:var(--ink);--button-hover-border:transparent;width:100%;min-height:32px;justify-content:flex-start;gap:10px;padding:6px 8px;border-radius:6px;font-size:12px;font-weight:500;line-height:1.25;letter-spacing:-.01em;transition:color .14s ease,background-color .14s ease}
.service-indicator span{width:6px;height:6px;border-radius:50%;background:var(--subtle);flex:0 0 auto;transition:background-color .16s ease}
.service-indicator.online span{background:var(--success)}
.service-indicator.recovering span{background:#b7791f}
.service-indicator.offline{--button-fg:var(--danger);--button-hover-fg:var(--danger)}
.service-indicator.offline span{background:var(--danger)}
.service-indicator.checking{--button-fg:var(--subtle)}
.service-indicator b{font:inherit;white-space:nowrap}
@media(max-width:600px){.service-indicators{flex-direction:row;flex-wrap:wrap;justify-content:center;gap:2px 8px}.service-indicator{width:auto;min-height:32px;padding:6px 8px;gap:8px;font-size:11px}}
</style>
