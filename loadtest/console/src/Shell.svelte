<script>
import {onMount} from 'svelte';
import Icon from './Icon.svelte';
import Brand from './Brand.svelte';
import ServiceIndicators from './ServiceIndicators.svelte';
import {page} from '$app/stores';
import {api} from './api.js';
export let data;
import {beforeNavigate,afterNavigate,goto} from '$app/navigation';
import UnsavedChanges from './UnsavedChanges.svelte';
import {navigationContext,rememberPage,sectionHref} from './navigation.mjs';
let settings=structuredClone(data.settings),savedSettings=JSON.stringify(settings),settingsDialog,pending='',error='',databaseCheck=null,serviceChecks={},certificates=[],workers=[],certificateFile,certificateName='';
let section='connections',settingsPanel,loading=false,leaveDialog,allowLeave=false,copied=false;
let restorePageScroll;
const sections=[['connections','Connections'],['database','Database & TLS'],['load','Load generation'],['workers','Workers']];
$: dirty=JSON.stringify(settings)!==savedSettings;
$: databaseFingerprint=JSON.stringify(settings.database);
$: if(databaseCheck&&databaseCheck.fingerprint!==databaseFingerprint)databaseCheck=null;
$: activeCertificate=JSON.parse(savedSettings).database.caCertificateId;
function selectSection(id){section=id;if(settingsPanel)settingsPanel.scrollTop=0;}
function settingsKey(event,index){if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End'].includes(event.key))return;event.preventDefault();const next=event.key==='Home'?0:event.key==='End'?sections.length-1:(index+(['ArrowRight','ArrowDown'].includes(event.key)?1:-1)+sections.length)%sections.length;selectSection(sections[next][0]);event.currentTarget.parentElement.querySelectorAll('button')[next].focus();}
function lockPageScroll(){
  if(restorePageScroll)return;
  const root=document.documentElement,body=document.body,scrollbar=Math.max(0,window.innerWidth-root.clientWidth);
  const previous={rootOverflow:root.style.overflow,bodyOverflow:body.style.overflow,bodyPaddingRight:body.style.paddingRight};
  root.style.overflow='hidden';body.style.overflow='hidden';
  if(scrollbar)body.style.paddingRight=`${Number.parseFloat(getComputedStyle(body).paddingRight||'0')+scrollbar}px`;
  restorePageScroll=()=>{root.style.overflow=previous.rootOverflow;body.style.overflow=previous.bodyOverflow;body.style.paddingRight=previous.bodyPaddingRight;restorePageScroll=null;};
}
async function openSettings(event){section=event?.detail?.section||'connections';if(settingsDialog.open)return;settingsDialog.showModal();lockPageScroll();loading=true;error='';try{settings=await api.settings();savedSettings=JSON.stringify(settings);await Promise.all([api.certificates().then(x=>certificates=x),api.workers().then(x=>workers=x)]);}catch(e){error=e.message;}finally{loading=false;}}
async function closeSettings(){if(pending)return;if(!dirty||await leaveDialog.ask()){if(dirty)settings=JSON.parse(savedSettings);settingsDialog.close();}}
async function saveSettings(){pending='save';error='';try{settings=await api.saveSettings(settings);savedSettings=JSON.stringify(settings);settingsDialog.close();window.dispatchEvent(new Event('console-connections-saved'));return true;}catch(e){error=e.message;return false;}finally{pending='';}}
async function testDatabase(){pending='test';databaseCheck=null;const fingerprint=databaseFingerprint;try{const result=await api.testDatabase(settings.database);if(databaseFingerprint===fingerprint)databaseCheck={...result,fingerprint};}catch(e){if(databaseFingerprint===fingerprint)databaseCheck={ok:false,message:e.message,fingerprint};}finally{pending='';}}
async function testService(kind,endpoint){pending=`test-${kind}`;serviceChecks={...serviceChecks,[kind]:null};try{serviceChecks={...serviceChecks,[kind]:{...await api.testService(kind,endpoint,kind==='superposition'?settings.superposition:undefined),endpoint}};}catch(e){serviceChecks={...serviceChecks,[kind]:{ok:false,message:e.message,endpoint}};}finally{pending='';}}
async function importCertificate(){if(!certificateFile)return;pending='certificate';error='';try{const added=await api.importCertificate(certificateFile,certificateName);certificates=[...certificates,added];settings.database.caCertificateId=added.id;settings.database.caPath='';certificateFile=null;certificateName='';}catch(e){error=e.message;}finally{pending='';}}
async function removeCertificate(id){pending='certificate';error='';try{await api.removeCertificate(id);certificates=certificates.filter(item=>item.id!==id);}catch(e){error=e.message;}finally{pending='';}}
async function deleteWorker(id){pending='worker';error='';try{await api.removeWorker(id);workers=workers.filter(worker=>worker.id!==id);}catch(e){error=e.message;}finally{pending='';}}
function workerCommand(){return `LOADTEST_CONTROL_PLANE_URL=${window.location.origin} LOADTEST_WORKER_NAME=worker-1 LOADTEST_WORKER_ENROLLMENT_TOKEN='<server enrollment token>' LOADTEST_WORKER_TPS=250 LOADTEST_WORKER_VUS=2500 npm run worker:agent`;}
async function copyWorker(){try{await navigator.clipboard.writeText(workerCommand());copied=true;setTimeout(()=>copied=false,2000);}catch{error='Clipboard unavailable. Select and copy the command below.';}}
afterNavigate(({to})=>{if(to)rememberPage(new URL(window.location.href));});
beforeNavigate(({cancel,to,type})=>{if(allowLeave){allowLeave=false;return;}if(!settingsDialog?.open)return;if(pending){cancel();return;}if(dirty){cancel();leaveDialog.ask().then(ok=>{if(ok&&to){if(dirty)settings=JSON.parse(savedSettings);settingsDialog.close();allowLeave=true;goto(to.url,{replaceState:type==='popstate'});}});}else settingsDialog.close();});
onMount(()=>{const open=e=>openSettings(e),unload=e=>{if(settingsDialog.open&&dirty){e.preventDefault();e.returnValue='';}},closed=()=>restorePageScroll?.();window.addEventListener('console-settings',open);window.addEventListener('beforeunload',unload);settingsDialog.addEventListener('close',closed);const poll=setInterval(()=>{if(settingsDialog.open&&section==='workers')api.workers().then(x=>workers=x).catch(e=>error=e.message);},5000);return()=>{clearInterval(poll);restorePageScroll?.();settingsDialog.removeEventListener('close',closed);window.removeEventListener('console-settings',open);window.removeEventListener('beforeunload',unload);};});
</script>
<header class="app-header">
  <a class="brand" href="/scenarios" aria-label="JUSPAY Hyperswitch · Testbench home"><img class="company-logo" src="/hyperswitch-logo.svg" alt="JUSPAY Hyperswitch"/><span class="product-brand"><Brand/></span></a>
  <div class="header-actions"><button class="header-button" onclick={openSettings}><Icon name="settings"/> Settings</button><form action="/logout" method="POST"><button class="header-button" aria-label="Sign out" title="Sign out"><span>Sign out</span><Icon name="logout"/></button></form></div>
</header>
<div class="console-frame">
  <nav class="section-nav" aria-label="Console sections"><div class="section-links">{#each ['scenarios','fixtures','performance','results'] as item}<a class:active={$page.url.pathname===`/${item}`||$page.url.pathname.startsWith(`/${item}/`)} href={sectionHref(item,$page.url,$navigationContext)}><Icon name={item}/><span>{item[0].toUpperCase()+item.slice(1)}</span></a>{/each}</div></nav>
  <aside class="service-sidebar" aria-labelledby="service-health-heading">
    <div class="service-health">
      <h2 id="service-health-heading">HEALTH</h2>
      <ServiceIndicators {settings}/>
    </div>
  </aside>
  <div class="console-content"><slot/></div>
</div>

<UnsavedChanges bind:this={leaveDialog} save={saveSettings}/>
<dialog oncancel={e=>{e.preventDefault();closeSettings();}} class="settings-dialog" bind:this={settingsDialog} aria-labelledby="settings-title">
  <section class="settings">
    <div class="section-head"><h1 id="settings-title">Settings</h1><button class="icon-button" aria-label="Close settings" onclick={closeSettings}><Icon name="close"/></button></div>
    {#if error}<div class="message error" role="alert">{error}</div>{/if}
    <div class="settings-body">
    <div class="settings-tabs" role="tablist" aria-label="Settings sections">{#each sections as [id,label],index}<button class="quiet" role="tab" tabindex={section===id?0:-1} onkeydown={e=>settingsKey(e,index)} aria-selected={section===id} class:active={section===id} onclick={()=>selectSection(id)}>{label}</button>{/each}</div>
    <div class="settings-grid" bind:this={settingsPanel}>
    {#if loading}<p role="status">Loading settings…</p>{:else}
      <div hidden={section!=='connections'}>
        <h2>Services</h2>
        <div class="fields"><div class="service-field"><label for="service-router">Router</label><div class="service-input"><input id="service-router" bind:value={settings.router}/><button type="button" class="secondary" class:check-success={serviceChecks.router?.endpoint===settings.router&&serviceChecks.router?.ok} class:check-failed={serviceChecks.router?.endpoint===settings.router&&serviceChecks.router&&!serviceChecks.router.ok} disabled={!!pending} onclick={()=>testService('router',settings.router)}>{pending==='test-router'?'Checking…':serviceChecks.router?.endpoint===settings.router?serviceChecks.router.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.router?.endpoint===settings.router}<small class:invalid={!serviceChecks.router.ok} role="status">{serviceChecks.router.message}</small>{/if}</div><div class="service-field"><label for="service-modularPm">Payment methods</label><div class="service-input"><input id="service-modularPm" bind:value={settings.modularPm}/><button type="button" class="secondary" class:check-success={serviceChecks.modularPm?.endpoint===settings.modularPm&&serviceChecks.modularPm?.ok} class:check-failed={serviceChecks.modularPm?.endpoint===settings.modularPm&&serviceChecks.modularPm&&!serviceChecks.modularPm.ok} disabled={!!pending} onclick={()=>testService('modularPm',settings.modularPm)}>{pending==='test-modularPm'?'Checking…':serviceChecks.modularPm?.endpoint===settings.modularPm?serviceChecks.modularPm.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.modularPm?.endpoint===settings.modularPm}<small class:invalid={!serviceChecks.modularPm.ok} role="status">{serviceChecks.modularPm.message}</small>{/if}</div><label>Router admin key<input type="password" bind:value={settings.adminApiKey} placeholder={settings.hasAdminApiKey ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Connector<input bind:value={settings.connector} /></label></div>
        <h2>Superposition</h2>
        <div class="fields"><div class="service-field"><label for="service-superposition">Endpoint</label><div class="service-input"><input id="service-superposition" bind:value={settings.superposition.endpoint}/><button type="button" class="secondary" class:check-success={serviceChecks.superposition?.endpoint===settings.superposition.endpoint&&serviceChecks.superposition?.ok} class:check-failed={serviceChecks.superposition?.endpoint===settings.superposition.endpoint&&serviceChecks.superposition&&!serviceChecks.superposition.ok} disabled={!!pending} onclick={()=>testService('superposition',settings.superposition.endpoint)}>{pending==='test-superposition'?'Checking…':serviceChecks.superposition?.endpoint===settings.superposition.endpoint?serviceChecks.superposition.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.superposition?.endpoint===settings.superposition.endpoint}<small class:invalid={!serviceChecks.superposition.ok} role="status">{serviceChecks.superposition.message}</small>{/if}</div><label>Authentication<select bind:value={settings.superposition.accessHeader}><option value="x-superposition-access">x-superposition-access header</option><option value="authorization">Authorization bearer token</option></select></label><label>Token<input type="password" bind:value={settings.superposition.token} placeholder={settings.superposition.hasToken ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Organization<input bind:value={settings.superposition.organizationId} /></label><label>Workspace<input bind:value={settings.superposition.workspaceId} /></label><label>Refresh timeout · seconds<input type="number" min="0" bind:value={settings.superposition.refreshSeconds}/></label><label>Poll every · seconds<input type="number" min="0.25" max="60" step="0.25" bind:value={settings.superposition.pollSeconds}/></label></div>
        <label>Defaults<textarea rows="5" bind:value={settings.superposition.defaultsJson}></textarea></label>
      </div><div hidden={section!=='load'}>
        <h2>Load generator</h2>
        <p>VU capacity applied when each load or smoke run starts.</p>
        <div class="fields"><label>Preallocated VUs per TPS<input type="number" min="0.1" step="0.1" bind:value={settings.preAllocatedVusMultiplier}/></label><label>Maximum VUs per TPS<input type="number" min="0.1" step="0.1" bind:value={settings.maxVusMultiplier}/></label></div>
      </div><div hidden={section!=='workers'}>
        <h2>Workers</h2><p>Load agents connect outbound and report their available TPS and VU capacity.</p>
        {#if !settings.workerEnrollmentConfigured}<div class="message error" role="alert">Worker enrollment is disabled. Set <code>LOADTEST_WORKER_ENROLLMENT_TOKEN</code> in <code>loadtest/console/.env</code>, then restart the console.</div>{/if}
        <details class="advanced"><summary>Connect a worker</summary><p>Run from loadtest/console on each load machine. Use a unique name and the enrollment token configured on the master.</p><pre class="onboarding">{typeof window!=='undefined'?workerCommand():''}</pre><button type="button" class="secondary" onclick={copyWorker}>{copied?'Copied':'Copy command'}</button><p>Fault commands run on the master. Workers generate load only. Health refreshes every 5 seconds.</p></details><div class="worker-list">{#each workers as worker}<div><span class:offline={worker.status==='offline'}></span><strong>{worker.name}</strong><small>{worker.status} · {worker.capacityTps} TPS · {worker.capacityVus} VUs</small>{#if worker.id!=='local'}<button class="icon-button danger" aria-label={`Remove ${worker.name}`} disabled={worker.status!=='offline'||!!pending} onclick={()=>deleteWorker(worker.id)}><Icon name="close"/></button>{/if}</div>{:else}<p>No workers registered.</p>{/each}</div>
      </div>
      <div hidden={section!=='database'}>
        <h2 id="database-settings">Database</h2><p>Use an existing PostgreSQL account with read access to payment_intent.</p>
        <div class="fields"><label>Host<input bind:value={settings.database.host} /></label><label>Port<input type="number" bind:value={settings.database.port} /></label><label>Database<input bind:value={settings.database.database} /></label><label>Read-only user<input bind:value={settings.database.user} /></label><label>Password<input type="password" bind:value={settings.database.password} placeholder={settings.database.hasPassword ? 'Configured · leave blank to keep' : 'Required'} /></label></div>
        <label>TLS<select bind:value={settings.database.tlsMode}><option value="disable">Disabled</option><option value="require">Required · encrypted</option><option value="verify">Verified certificate</option></select></label>
        {#if settings.database.tlsMode==='verify'}
          <label>Certificate authority<select bind:value={settings.database.caCertificateId} onchange={()=>{if(settings.database.caCertificateId)settings.database.caPath='';}}><option value="">System trust store</option>{#each certificates as certificate}<option value={certificate.id}>{certificate.name} · {certificate.fingerprint?.slice(-17)}</option>{/each}</select></label>
          <p role="status">{settings.database.caCertificateId!==activeCertificate?'Certificate selection pending · Save settings to activate':settings.database.caCertificateId?'Saved certificate is active':'Using the system trust store'}</p><details class="advanced"><summary>Import CA certificate</summary><div class="fields"><label>Name<input bind:value={certificateName} placeholder="Database CA" /></label><label>PEM or CRT file<input type="file" accept=".pem,.crt,application/x-pem-file,application/pem-certificate-chain" onchange={e=>certificateFile=e.currentTarget.files?.[0]}/></label></div><button type="button" class="secondary" disabled={!certificateFile||!!pending} onclick={importCertificate}>{pending==='certificate'?'Importing…':'Import certificate'}</button>{#if certificates.length}<div class="certificate-list">{#each certificates as certificate}<div><span><strong>{certificate.name}</strong><small>{certificate.subject} · expires {new Date(certificate.validTo).toLocaleDateString()}</small></span><button type="button" class="icon-button danger" aria-label={`Delete ${certificate.name}`} disabled={settings.database.caCertificateId===certificate.id||activeCertificate===certificate.id||!!pending} onclick={()=>removeCertificate(certificate.id)}><Icon name="close"/></button></div>{/each}</div>{/if}</details>
          <details class="advanced"><summary>Server-mounted certificate</summary><label>Absolute CA path<input bind:value={settings.database.caPath} disabled={Boolean(settings.database.caCertificateId)} placeholder="Optional · for mounted secrets" /></label></details>
        {/if}
        <button type="button" class="secondary database-test" class:check-success={databaseCheck?.ok} class:check-failed={databaseCheck&&!databaseCheck.ok} disabled={!!pending} onclick={testDatabase}>{pending==='test'?'Checking…':databaseCheck?databaseCheck.ok?'Succeeded':'Failed':'Test connection'}</button>
        {#if databaseCheck}<p role="status" class:invalid={!databaseCheck.ok}>{databaseCheck.message}</p>{/if}
      </div>
    {/if}
    </div>
    </div>
    <div class="actions settings-save"><small>{dirty?'Unsaved changes':'All changes saved'}</small><button type="button" class="secondary" onclick={closeSettings}>Close</button><button type="button" disabled={!!pending || loading || !dirty} onclick={saveSettings}>{pending === 'save' ? 'Saving…' : 'Save settings'}</button></div>
  </section>
</dialog>

<style>
.brand{gap:14px}.company-logo{display:block;width:175px;height:auto}.product-brand{display:flex;align-items:center;padding-left:14px;border-left:1px solid var(--border)}
@media(max-width:900px){.company-logo{width:150px}.product-brand{padding-left:12px}}
@media(max-width:600px){.company-logo{width:138px}.product-brand{display:none}}
.service-input{display:flex;align-items:center;gap:8px;margin-top:6px}.service-input input{margin:0;min-width:0}.service-input button{flex-shrink:0}.database-test{margin-top:16px}.service-field small{display:block;margin-top:6px;font-size:11px}
.settings{display:flex;flex-direction:column;height:100%;gap:20px;overflow:hidden}.settings>.section-head,.settings>.message,.settings>.settings-save{flex-shrink:0}.settings>.message{margin:0;max-height:90px;overflow:auto}.settings-body{display:grid;grid-template-columns:156px minmax(0,1fr);gap:24px;min-height:0;flex:1}.settings>.section-head{margin:0;padding-bottom:20px;border-bottom:1px solid var(--border);align-items:flex-start}.settings-tabs{display:flex;flex-direction:column;align-items:stretch;gap:5px;overflow-y:auto;padding:4px}.settings-tabs button{justify-content:flex-start;text-align:left;white-space:normal;font-size:12px;padding:10px 12px}.settings-tabs button.active{--button-bg:var(--accent-soft);--button-fg:var(--accent);--button-hover-bg:var(--selected-hover);--button-hover-fg:var(--accent)}.settings-grid{display:block;min-height:0;overflow-y:auto;overscroll-behavior:contain;scrollbar-gutter:stable;padding:4px 8px 8px 24px;border-left:1px solid var(--border)}.settings-save{background:var(--surface);padding:16px 0 0;margin:0;border-top:1px solid var(--border)}.settings-save small{margin-right:auto;color:var(--muted);font-size:12px}.onboarding{white-space:pre-wrap;overflow-wrap:anywhere;font-size:12px;background:var(--surface-muted);padding:14px;border-radius:8px}[hidden]{display:none!important}.certificate-list,.worker-list{display:grid;gap:8px;margin-top:12px}.certificate-list>div,.worker-list>div{display:flex;align-items:center;gap:10px;border:1px solid var(--border);border-radius:7px;padding:10px 12px}.certificate-list>div{justify-content:space-between}.certificate-list span{display:grid;gap:3px;min-width:0}.certificate-list small{overflow-wrap:anywhere}.worker-list span{width:7px;height:7px;border-radius:50%;background:var(--success);flex-shrink:0}.worker-list span.offline{background:var(--subtle)}.worker-list small{margin-left:auto;color:var(--muted)}
@media(max-width:650px){.settings{gap:16px}.settings-body{display:flex;flex-direction:column;gap:12px}.settings-tabs{flex-direction:row;flex-wrap:wrap;flex-shrink:0;gap:4px}.settings-tabs button{padding:8px 10px}.settings-grid{border:0;padding:4px 4px 8px;flex:1}.settings-save{flex-wrap:wrap}.settings-save small{flex-basis:100%}.worker-list>div{flex-wrap:wrap}}
</style>
