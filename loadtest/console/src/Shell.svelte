<script>
import {environments,environmentLabel} from './environments.mjs';
import {onMount} from 'svelte';
import Icon from './Icon.svelte';
import Brand from './Brand.svelte';
import ServiceIndicators from './ServiceIndicators.svelte';
import {page,navigating} from '$app/stores';
import {api} from './api.js';
export let data;
import {beforeNavigate,afterNavigate,goto} from '$app/navigation';
import UnsavedChanges from './UnsavedChanges.svelte';
import {defaultResultsLimits,resultsLimitFields} from './results-limits.mjs';
import {navigationContext,rememberPage,sectionHref} from './navigation.mjs';
let settings=structuredClone(data.settings),savedSettings=JSON.stringify(settings),settingsDialog,pending='',error='',databaseCheck=null,serviceChecks={},certificates=[],workers=[],certificateFile,certificateName='';
let section='connections',settingsPanel,loading=false,leaveDialog,allowLeave=false,copied=false;
let restorePageScroll;
$: loadingSection=$navigating?.to?.url.pathname!==$page.url.pathname&&['/scenarios','/fixtures','/performance','/results'].includes($navigating?.to?.url.pathname)?$navigating.to.url.pathname.slice(1):'';
const sections=[['environment','Environment'],['connections','Connections'],['database','Database & TLS'],['load','Load generation'],['results','Results limits'],['workers','Workers']];
$: dirty=JSON.stringify(settings)!==savedSettings;
$: healthSettings=JSON.parse(savedSettings);
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
async function openSettings(event){section=event?.detail?.section||'connections';if(settingsDialog.open)return;settingsDialog.showModal();lockPageScroll();loading=true;error='';try{settings=await api.settings();environmentDrafts={};savedSettings=JSON.stringify(settings);await Promise.all([api.certificates().then(x=>certificates=x),api.workers().then(x=>workers=x)]);}catch(e){error=e.message;}finally{loading=false;}}
let environmentDrafts={};
async function selectEnvironment(environment){environmentDrafts[settings.environment]=structuredClone(settings);pending='environment';error='';try{settings=environmentDrafts[environment]||await api.settings(environment);databaseCheck=null;serviceChecks={};}catch(e){error=e.message;}finally{pending='';}}
async function closeSettings(){if(pending)return;if(!dirty||await leaveDialog.ask()){if(dirty)settings=JSON.parse(savedSettings);settingsDialog.close();}}
async function saveSettings(){pending='save';error='';try{settings=await api.saveSettings(settings);savedSettings=JSON.stringify(settings);settingsDialog.close();window.dispatchEvent(new Event('console-connections-saved'));return true;}catch(e){error=e.message;return false;}finally{pending='';}}
async function testDatabase(){pending='test';databaseCheck=null;const fingerprint=databaseFingerprint;try{const result=await api.testDatabase(settings.database,settings.environment);if(databaseFingerprint===fingerprint)databaseCheck={...result,fingerprint};}catch(e){if(databaseFingerprint===fingerprint)databaseCheck={ok:false,message:e.message,fingerprint};}finally{pending='';}}
async function testService(kind,endpoint){pending=`test-${kind}`;serviceChecks={...serviceChecks,[kind]:null};try{serviceChecks={...serviceChecks,[kind]:{...await api.testService(kind,endpoint,kind==='superposition'?settings.superposition:undefined,settings.environment),endpoint}};}catch(e){serviceChecks={...serviceChecks,[kind]:{ok:false,message:e.message,endpoint}};}finally{pending='';}}
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
  <div class="brand-group"><a class="brand" href="/scenarios" aria-label="JUSPAY Hyperswitch · Testbench home"><img class="company-logo" src="/hyperswitch-logo.svg" alt="JUSPAY Hyperswitch"/><span class="product-brand"><Brand/></span></a><button type="button" class="environment-label" aria-label={`Change environment: ${environmentLabel(healthSettings.environment)}`} aria-haspopup="dialog" onclick={()=>openSettings({detail:{section:'environment'}})}>{environmentLabel(healthSettings.environment)}</button></div>
  <div class="header-actions"><button class="header-button" onclick={openSettings}><Icon name="settings"/> Settings</button><form action="/logout" method="POST"><button class="header-button" aria-label="Sign out" title="Sign out"><span>Sign out</span><Icon name="logout"/></button></form></div>
</header>
<div class="console-frame">
  <nav class="section-nav" aria-label="Console sections" aria-busy={!!$navigating}><div class="section-links">{#each ['scenarios','fixtures','performance','results'] as item}<a class:active={$page.url.pathname===`/${item}`||$page.url.pathname.startsWith(`/${item}/`)} class:opening={$navigating?.to?.url.pathname===`/${item}`} aria-current={$page.url.pathname===`/${item}`?'page':undefined} href={sectionHref(item,$page.url,$navigationContext)}><Icon name={item}/><span>{item[0].toUpperCase()+item.slice(1)}</span></a>{/each}</div></nav>
  <aside class="service-sidebar" aria-labelledby="service-health-heading">
    <div class="service-health">
      <h2 id="service-health-heading">HEALTH</h2>
      <ServiceIndicators settings={healthSettings}/>
    </div>
  </aside>
  <div class="console-content" aria-busy={!!loadingSection}>
    <div hidden={!!loadingSection}><slot/></div>
    {#if loadingSection}<section class="section-loader" role="status" aria-live="polite"><span class="loading-spinner" aria-hidden="true"></span><p>Loading {loadingSection}…</p></section>{/if}
  </div>
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
      <div hidden={section!=='environment'}>
        <h2>Environment</h2>
        <p>Select where connections, load tests and faults run.</p>
        <label>Provider<select value={settings.environment} disabled={!!pending} onchange={e=>selectEnvironment(e.currentTarget.value)}><option value="unassigned" disabled>Choose an environment</option>{#each environments as environment}<option value={environment.id}>{environment.label}</option>{/each}</select></label>
      </div>
      <div hidden={section!=='connections'}>
        <h2>Services</h2>
        <div class="fields"><div class="service-field"><label for="service-router">Router</label><div class="service-input"><input id="service-router" bind:value={settings.router}/><button type="button" class="secondary" class:check-success={serviceChecks.router?.endpoint===settings.router&&serviceChecks.router?.ok} class:check-failed={serviceChecks.router?.endpoint===settings.router&&serviceChecks.router&&!serviceChecks.router.ok} disabled={!!pending||!environments.find(e=>e.id===settings.environment)?.supported} onclick={()=>testService('router',settings.router)}>{pending==='test-router'?'Checking…':serviceChecks.router?.endpoint===settings.router?serviceChecks.router.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.router?.endpoint===settings.router}<small class:invalid={!serviceChecks.router.ok} role="status">{serviceChecks.router.message}</small>{/if}</div><div class="service-field"><label for="service-modularPm">Payment methods</label><div class="service-input"><input id="service-modularPm" bind:value={settings.modularPm}/><button type="button" class="secondary" class:check-success={serviceChecks.modularPm?.endpoint===settings.modularPm&&serviceChecks.modularPm?.ok} class:check-failed={serviceChecks.modularPm?.endpoint===settings.modularPm&&serviceChecks.modularPm&&!serviceChecks.modularPm.ok} disabled={!!pending||!environments.find(e=>e.id===settings.environment)?.supported} onclick={()=>testService('modularPm',settings.modularPm)}>{pending==='test-modularPm'?'Checking…':serviceChecks.modularPm?.endpoint===settings.modularPm?serviceChecks.modularPm.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.modularPm?.endpoint===settings.modularPm}<small class:invalid={!serviceChecks.modularPm.ok} role="status">{serviceChecks.modularPm.message}</small>{/if}</div><label>Router admin key<input type="password" bind:value={settings.adminApiKey} placeholder={settings.hasAdminApiKey ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Connector<input bind:value={settings.connector} /></label></div>
        <h2>Superposition</h2>
        <div class="fields"><div class="service-field"><label for="service-superposition">Endpoint</label><div class="service-input"><input id="service-superposition" bind:value={settings.superposition.endpoint}/><button type="button" class="secondary" class:check-success={serviceChecks.superposition?.endpoint===settings.superposition.endpoint&&serviceChecks.superposition?.ok} class:check-failed={serviceChecks.superposition?.endpoint===settings.superposition.endpoint&&serviceChecks.superposition&&!serviceChecks.superposition.ok} disabled={!!pending||!environments.find(e=>e.id===settings.environment)?.supported} onclick={()=>testService('superposition',settings.superposition.endpoint)}>{pending==='test-superposition'?'Checking…':serviceChecks.superposition?.endpoint===settings.superposition.endpoint?serviceChecks.superposition.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.superposition?.endpoint===settings.superposition.endpoint}<small class:invalid={!serviceChecks.superposition.ok} role="status">{serviceChecks.superposition.message}</small>{/if}</div><label>Authentication<select bind:value={settings.superposition.accessHeader}><option value="x-superposition-access">x-superposition-access header</option><option value="authorization">Authorization bearer token</option></select></label><label>Token<input type="password" bind:value={settings.superposition.token} placeholder={settings.superposition.hasToken ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Organization<input bind:value={settings.superposition.organizationId} /></label><label>Workspace<input bind:value={settings.superposition.workspaceId} /></label><label>Refresh timeout · seconds<input type="number" min="0" bind:value={settings.superposition.refreshSeconds}/></label><label>Poll every · seconds<input type="number" min="0.25" max="60" step="0.25" bind:value={settings.superposition.pollSeconds}/></label></div>
        <label>Defaults<textarea rows="5" bind:value={settings.superposition.defaultsJson}></textarea></label>
      </div><div hidden={section!=='load'}>
        <h2>Load generator</h2>
        <p>VU capacity applied when each load or smoke run starts.</p>
        <div class="fields"><label>Preallocated VUs per TPS<input type="number" min="0.1" step="0.1" bind:value={settings.preAllocatedVusMultiplier}/></label><label>Maximum VUs per TPS<input type="number" min="0.1" step="0.1" bind:value={settings.maxVusMultiplier}/></label></div>
      </div><div hidden={section!=='results'}>
        <h2>Results limits</h2>
        <p>Applied to future runs. Each run keeps its own copy of these limits.</p>
        <div class="fields">{#each resultsLimitFields as [key,label,unit,max]}<label for={`results-${key}`}>{label} · {unit}<input id={`results-${key}`} type="number" min={key==='recoveryWindowSeconds'?0.1:0} {max} step={key==='maximumLostPayments'?1:'any'} bind:value={settings.resultsLimits[key]} required/></label>{/each}</div>
        <button type="button" class="secondary" onclick={()=>settings.resultsLimits={...defaultResultsLimits}}>Reset to defaults</button>
      </div><div hidden={section!=='workers'}>
        <h2>Workers</h2><p>Load agents connect outbound and report their available TPS and VU capacity.</p>
        {#if !settings.workerEnrollmentConfigured}<div class="message error" role="alert">Worker enrollment is disabled. Set <code>LOADTEST_WORKER_ENROLLMENT_TOKEN</code> in <code>loadtest/console/.env</code>, then restart the console.</div>{/if}
        <details class="advanced"><summary>Connect a worker</summary><p>Run from loadtest/console on each load machine. Use a unique name and the enrollment token configured on the master.</p><pre class="onboarding">{typeof window!=='undefined'?workerCommand():''}</pre><button type="button" class="secondary" onclick={copyWorker}>{copied?'Copied':'Copy command'}</button><p>Fault commands run on the master. Workers generate load only. Health refreshes every 5 seconds.</p></details><div class="worker-list">{#each workers as worker}<div><span class:offline={worker.status==='offline'}></span><strong>{worker.name}</strong><small>{worker.status} · {worker.capacityTps} TPS · {worker.capacityVus} VUs</small>{#if worker.id!=='local'}<button class="icon-button danger" aria-label={`Remove ${worker.name}`} disabled={worker.status!=='offline'||!!pending} onclick={()=>deleteWorker(worker.id)}><Icon name="close"/></button>{/if}</div>{:else}<p>No workers registered.</p>{/each}</div>
      </div>
      <div hidden={section!=='database'}>
        <h2 id="database-settings">Database</h2><p>Use an existing PostgreSQL account with read access to payment_intent.</p>
        <div class="fields">
          <label>Host<input bind:value={settings.database.host} /></label>
          <label>Port<input type="number" bind:value={settings.database.port} /></label>
          <label>Database<input bind:value={settings.database.database} /></label>
          <label>Read-only user<input bind:value={settings.database.user} /></label>
          <label>Password<input type="password" bind:value={settings.database.password} placeholder={settings.database.hasPassword ? 'Configured · leave blank to keep' : 'Required'} /></label>
          <label>TLS<select bind:value={settings.database.tlsMode}><option value="disable">Disabled</option><option value="require">Required · encrypted</option><option value="verify">Verified certificate</option></select></label>
        </div>
        {#if settings.database.tlsMode==='verify'}
          <details class="custom-ca">
            <summary>Custom CA certificate <span>{settings.database.caCertificateId ? certificates.find(c=>c.id===settings.database.caCertificateId)?.name || 'Imported certificate' : settings.database.caPath ? 'Server-mounted certificate' : 'System trust store'}</span></summary>
            <div class="ca-options">
              {#if certificates.length}
                <label>Certificate authority<select bind:value={settings.database.caCertificateId} onchange={()=>{if(settings.database.caCertificateId)settings.database.caPath='';}}><option value="">System trust store</option>{#each certificates as certificate}<option value={certificate.id}>{certificate.name} · {certificate.fingerprint?.slice(-17)}</option>{/each}</select></label>
              {/if}
              <div class="fields">
                <label>Certificate name<input bind:value={certificateName} placeholder="Database CA" /></label>
                <label>PEM or CRT file<input type="file" accept=".pem,.crt,application/x-pem-file,application/pem-certificate-chain" onchange={e=>certificateFile=e.currentTarget.files?.[0]}/></label>
              </div>
              <button type="button" class="secondary" disabled={!certificateFile||!!pending} onclick={importCertificate}>{pending==='certificate'?'Importing…':'Import certificate'}</button>
              {#if certificates.length}
                <div class="certificate-list">{#each certificates as certificate}<div><span><strong>{certificate.name}</strong><small>{certificate.subject} · expires {new Date(certificate.validTo).toLocaleDateString()}</small></span><button type="button" class="icon-button danger" aria-label={`Delete ${certificate.name}`} disabled={settings.database.caCertificateId===certificate.id||activeCertificate===certificate.id||!!pending} onclick={()=>removeCertificate(certificate.id)}><Icon name="close"/></button></div>{/each}</div>
              {/if}
            </div>
          </details>
        {/if}
        <button type="button" class="secondary database-test" class:check-success={databaseCheck?.ok} class:check-failed={databaseCheck&&!databaseCheck.ok} disabled={!!pending||!environments.find(e=>e.id===settings.environment)?.supported} onclick={testDatabase}>{pending==='test'?'Checking…':databaseCheck?databaseCheck.ok?'Succeeded':'Failed':'Test connection'}</button>
        {#if databaseCheck}<p role="status" class:invalid={!databaseCheck.ok}>{databaseCheck.message}</p>{/if}
      </div>
    {/if}
    </div>
    </div>
    <div class="actions settings-save"><small>{dirty?'Unsaved changes':'All changes saved'}</small><button type="button" class="secondary" onclick={closeSettings}>Close</button><button type="button" disabled={!!pending || loading || !dirty} onclick={saveSettings}>{pending === 'save' ? 'Saving…' : 'Save settings'}</button></div>
  </section>
</dialog>

<style>
.section-nav a.opening{color:var(--accent);background:var(--accent-soft)}.section-loader{min-height:calc(100dvh - var(--header-height) - var(--nav-height));display:flex;align-items:center;justify-content:center;flex-direction:column;gap:14px;padding:32px;color:var(--muted)}.section-loader p{margin:0;font-size:14px}.loading-spinner{width:28px;height:28px;border:3px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:section-spin .8s linear infinite}@keyframes section-spin{to{transform:rotate(360deg)}}@media(prefers-reduced-motion:reduce){.loading-spinner{animation:none}}
.brand-group{display:flex;align-items:center;gap:14px;flex-shrink:0}.environment-label{--button-hover-bg:var(--accent-soft);--button-hover-fg:var(--accent);--button-hover-border:var(--accent);min-height:0;font-size:11px;font-weight:400;line-height:1.2;color:var(--accent);background:var(--accent-soft);border:1px solid transparent;border-radius:5px;padding:4px 8px;white-space:nowrap}.brand{gap:14px}.company-logo{display:block;width:175px;height:auto}.product-brand{display:flex;align-items:center;padding-left:14px;border-left:1px solid var(--border)}
@media(max-width:900px){.company-logo{width:150px}.product-brand{padding-left:12px}}
@media(max-width:600px){.company-logo{width:138px}.product-brand{display:none}}
.custom-ca{border-top:1px solid var(--border);padding-top:14px}.custom-ca summary{flex-wrap:wrap}.custom-ca summary>span{margin-left:auto;font-size:12px;font-weight:400;color:var(--muted)}.ca-options{display:grid;gap:14px;padding-top:16px}.ca-options>button{justify-self:start}.ca-options .certificate-list{margin-top:0}
.service-input{display:flex;align-items:center;gap:8px;margin-top:6px}.service-input input{margin:0;min-width:0}.service-input button{flex-shrink:0}.database-test{margin-top:16px}.service-field small{display:block;margin-top:6px;font-size:11px}
.settings{display:flex;flex-direction:column;height:100%;gap:20px;overflow:hidden}.settings>.section-head,.settings>.message,.settings>.settings-save{flex-shrink:0}.settings>.message{margin:0;max-height:90px;overflow:auto}.settings-body{display:grid;grid-template-columns:156px minmax(0,1fr);gap:24px;min-height:0;flex:1}.settings>.section-head{margin:0;padding-bottom:20px;border-bottom:1px solid var(--border);align-items:flex-start}.settings-tabs{display:flex;flex-direction:column;align-items:stretch;gap:5px;overflow-y:auto;padding:4px}.settings-tabs button{justify-content:flex-start;text-align:left;white-space:normal;font-size:12px;padding:10px 12px}.settings-tabs button.active{--button-bg:var(--accent-soft);--button-fg:var(--accent);--button-hover-bg:var(--selected-hover);--button-hover-fg:var(--accent)}.settings-grid{display:block;min-height:0;overflow-y:auto;overscroll-behavior:contain;scrollbar-gutter:stable;padding:4px 8px 8px 24px;border-left:1px solid var(--border)}.settings-save{background:var(--surface);padding:16px 0 0;margin:0;border-top:1px solid var(--border)}.settings-save small{margin-right:auto;color:var(--muted);font-size:12px}.onboarding{white-space:pre-wrap;overflow-wrap:anywhere;font-size:12px;background:var(--surface-muted);padding:14px;border-radius:8px}[hidden]{display:none!important}.certificate-list,.worker-list{display:grid;gap:8px;margin-top:12px}.certificate-list>div,.worker-list>div{display:flex;align-items:center;gap:10px;border:1px solid var(--border);border-radius:7px;padding:10px 12px}.certificate-list>div{justify-content:space-between}.certificate-list span{display:grid;gap:3px;min-width:0}.certificate-list small{overflow-wrap:anywhere}.worker-list span{width:7px;height:7px;border-radius:50%;background:var(--success);flex-shrink:0}.worker-list span.offline{background:var(--subtle)}.worker-list small{margin-left:auto;color:var(--muted)}
@media(max-width:650px){.settings{gap:16px}.settings-body{display:flex;flex-direction:column;gap:12px}.settings-tabs{flex-direction:row;flex-wrap:wrap;flex-shrink:0;gap:4px}.settings-tabs button{padding:8px 10px}.settings-grid{border:0;padding:4px 4px 8px;flex:1}.settings-save{flex-wrap:wrap}.settings-save small{flex-basis:100%}.worker-list>div{flex-wrap:wrap}}
</style>
