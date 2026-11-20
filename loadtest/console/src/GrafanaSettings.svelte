<script>
import {onDestroy} from 'svelte';
import {api} from './api.js';
import InfoTooltip from './InfoTooltip.svelte';
export let value;
export let environment;
export let disabled=false;
let adding=false,input='',busy=false,connectionBusy=false,connectionResult=null,error='',preview=null,viewing=-1,loaded=false,timer,controller,sequence=0,previousFingerprint;
$: fingerprint=JSON.stringify([value.baseUrl,value.token||'',environment]);
$: if(previousFingerprint!==fingerprint){previousFingerprint=fingerprint;connectionResult=null;clearPreview();if(adding&&input.trim())schedule(input);}
function clearPreview(){controller?.abort();sequence++;preview=null;error='';busy=false;loaded=false;viewing=-1;}
function schedule(url){clearTimeout(timer);clearPreview();if(url.trim())timer=setTimeout(()=>resolve(url,-1),500);}
async function resolve(url,index){
 clearTimeout(timer);clearPreview();viewing=index;busy=true;const current=++sequence;controller=new AbortController();
 try{const result=await api.previewGrafana(url,{baseUrl:value.baseUrl,token:value.token||''},environment,controller.signal);if(current===sequence){preview=result;loaded=false;if(index<0&&value.panels.some(p=>p.url===result.panel.url))error='This panel is already added';}}
 catch(e){if(current===sequence&&e.name!=='AbortError')error=e.message;}
 finally{if(current===sequence)busy=false;}
}
async function testConnection(){connectionBusy=true;connectionResult=null;const current=fingerprint;try{const result=await api.testGrafana({baseUrl:value.baseUrl,token:value.token||''},environment);if(current===fingerprint)connectionResult=result;}catch(e){if(current===fingerprint)connectionResult={ok:false,message:e.message};}finally{connectionBusy=false;}}
function view(index){if(viewing===index){clearPreview();return;}adding=false;input='';resolve(value.panels[index].url,index);}
function remove(index){value={...value,panels:value.panels.filter((_,i)=>i!==index)};if(adding&&input.trim())schedule(input);else clearPreview();}
function add(){if(!preview)return;if(value.panels.some(p=>p.url===preview.panel.url)){error='This panel is already added';return;}if(value.panels.length>=20){error='Choose up to 20 panels';return;}value={...value,panels:[...value.panels,preview.panel]};adding=false;input='';resolve(value.panels.at(-1).url,value.panels.length-1);}
function cancel(){adding=false;input='';clearTimeout(timer);clearPreview();}
onDestroy(()=>{clearTimeout(timer);controller?.abort();sequence++;});
</script>
<div class="grafana-settings">
 <h2>Grafana <InfoTooltip id="grafana-info" label="Grafana connection information" text="Use a server reachable from the console host and a Viewer service-account token. Embedded panels use an isolated origin; credentials remain on the server."/></h2>
 <div class="connection">
  <label>Server URL<input type="url" value={value.baseUrl} oninput={e=>value={...value,baseUrl:e.currentTarget.value}} placeholder="https://grafana.example.com" disabled={disabled}/></label>
  <label>Service-account token<input type="password" autocomplete="new-password" value={value.token||''} oninput={e=>value={...value,token:e.currentTarget.value}} placeholder={value.hasToken?'Configured · leave blank to keep':'Grafana Viewer token'} disabled={disabled}/></label>
  <div class="connection-test"><button type="button" class="secondary" disabled={disabled||connectionBusy||!value.baseUrl} onclick={testConnection}>{connectionBusy?'Connecting…':'Test connection'}</button>{#if connectionResult}<span class:invalid={!connectionResult.ok} class:verified={connectionResult.ok} role="status">{connectionResult.message}</span>{/if}</div>
 </div>
 <div class="panels-heading"><div><h3>Panels</h3><p>Shown in Performance when Grafana is selected.</p></div><button type="button" class="secondary" disabled={disabled||adding||value.panels.length>=20} onclick={()=>{clearPreview();adding=true;}}>+ Add panel</button></div>
 {#if value.panels.length}<div class="panel-list">{#each value.panels as panel,index (panel.url)}<div class="panel-row"><div class="panel-summary"><strong>{panel.title}</strong>{#if panel.dashboardTitle}<small>{panel.dashboardTitle}</small>{/if}</div><div class="panel-actions"><button type="button" class="quiet" aria-expanded={viewing===index} disabled={disabled} onclick={()=>view(index)}>{viewing===index?'Hide':'View'}</button><button type="button" class="quiet delete" aria-label={`Delete ${panel.title}`} disabled={disabled} onclick={()=>remove(index)}>Delete</button></div></div>
 {#if viewing===index}<div class="row-preview">{#if busy}<div class="preview-skeleton" role="status">Loading panel…</div>{:else if error}<p class="invalid" role="alert">{error}</p>{:else if preview}<div class="preview-frame" class:loading={!loaded}>{#if !loaded}<div class="preview-skeleton" role="status">Loading panel…</div>{/if}<iframe title={`Preview: ${panel.title}`} src={preview.previewUrl} sandbox="allow-scripts allow-same-origin" referrerpolicy="no-referrer" onload={()=>loaded=true}></iframe></div>{/if}</div>{/if}
 {/each}</div>{:else}<p class="empty">No panels added yet.</p>{/if}
 {#if adding}<div class="add-panel"><label>Panel URL<input type="url" bind:value={input} oninput={e=>schedule(e.currentTarget.value)} placeholder="Paste a Grafana panel link" disabled={disabled}/></label>
 {#if busy}<div class="preview-skeleton" role="status">Finding panel…</div>{:else if preview}<div class="preview-frame" class:loading={!loaded}>{#if !loaded}<div class="preview-skeleton" role="status">Loading panel…</div>{/if}<iframe title={`Preview: ${preview.panel.title}`} src={preview.previewUrl} sandbox="allow-scripts allow-same-origin" referrerpolicy="no-referrer" onload={()=>loaded=true}></iframe></div>{/if}
 {#if error}<p class="invalid" role="alert">{error}</p>{/if}<div class="add-actions"><button type="button" class="quiet" onclick={cancel}>Cancel</button><button type="button" disabled={disabled||busy||!preview||!!error} onclick={add}>Add panel</button></div></div>{/if}
</div>
<style>
.grafana-settings{max-width:880px}.connection{max-width:680px}.connection label{display:block;margin:18px 0}.connection-test{display:flex;align-items:center;gap:14px;flex-wrap:wrap}.connection-test span{font-size:12px}.verified{color:var(--success)}.panels-heading{display:flex;align-items:center;justify-content:space-between;gap:16px;margin:30px 0 14px;padding-top:22px;border-top:1px solid var(--border)}h3{margin:0;font-size:14px}.panels-heading p{margin:5px 0 0;font-size:12px;color:var(--muted)}.panel-list{border:1px solid var(--border);border-radius:9px;overflow:hidden}.panel-row{display:flex;align-items:center;justify-content:space-between;padding:14px 16px;gap:16px;border-top:1px solid var(--border)}.panel-row:first-child{border-top:0}.panel-summary{min-width:0}.panel-summary strong{display:block;font-size:13px;font-weight:550;overflow-wrap:anywhere}.panel-summary small{display:block;margin-top:4px;font-size:11px;color:var(--muted)}.panel-actions{display:flex;align-items:center;gap:8px;flex-shrink:0}.delete{color:var(--danger);--button-fg:var(--danger)}.row-preview{padding:0 16px 16px}.preview-frame{position:relative;height:300px;border:1px solid var(--border);border-radius:8px;overflow:hidden;background:var(--surface)}iframe{width:100%;height:100%;border:0;display:block}.preview-frame.loading iframe{opacity:0}.preview-skeleton{height:220px;display:grid;place-items:center;color:var(--muted);font-size:12px;background:linear-gradient(110deg,var(--surface-muted) 25%,var(--surface) 45%,var(--surface-muted) 65%);background-size:200% 100%;animation:shimmer 1.5s linear infinite;border-radius:8px}.preview-frame .preview-skeleton{position:absolute;inset:0;height:auto}.add-panel{margin-top:16px;padding:18px;border:1px solid var(--border);border-radius:9px;background:var(--surface-muted)}.add-panel label{display:block;margin:0 0 16px}.add-actions{display:flex;justify-content:flex-end;gap:8px;margin-top:16px}.invalid{font-size:12px;overflow-wrap:anywhere}.empty{font-size:12px;color:var(--muted);padding:20px 0}@keyframes shimmer{to{background-position:-200% 0}}@media(prefers-reduced-motion:reduce){.preview-skeleton{animation:none}}@media(max-width:600px){.panels-heading{align-items:flex-start}.panel-row{padding:12px;gap:8px}.panel-actions{gap:0}.row-preview{padding:0 12px 12px}.preview-frame{height:260px}}
</style>
