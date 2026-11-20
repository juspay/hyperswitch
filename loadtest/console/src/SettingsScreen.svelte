<script>
import GrafanaSettings from './GrafanaSettings.svelte';
import InfoTooltip from './InfoTooltip.svelte';
import Icon from './Icon.svelte';
import {environmentAccessNote,environments} from './environments.mjs';
import {onMount,tick} from 'svelte';
import {beforeNavigate,goto,invalidate} from '$app/navigation';
import {page} from './app-state.mjs';
import {api} from './api.js';
import {settingsSections as sections,settingsSection,settingsHref} from './settings-sections.mjs';
import {defaultResultsLimits,resultsLimitFields} from './results-limits.mjs';
export let data;
let settings=structuredClone(data.settings),savedSettings=JSON.stringify(settings),pending='',error='',notice='',databaseCheck=null,serviceChecks={},certificates=data.certificates,workers=data.workers,certificateFile,certificateName='';
let errorBanner,leaveNotice,allowLeave=false,copied=false,workerInstructions='',environmentDrafts={},confirmation='';
/** @type {{url:URL,replace:boolean}|null} */
let leaveTarget=null;
$: section=settingsSection($page.url.searchParams.get('section'));
$: dirty=JSON.stringify(settings)!==savedSettings;
$: if(dirty)notice='';
$: if(!dirty){leaveTarget=null;confirmation='';}
$: databaseFingerprint=JSON.stringify(settings.database);
$: if(databaseCheck&&databaseCheck.fingerprint!==databaseFingerprint)databaseCheck=null;
$: activeCertificate=JSON.parse(savedSettings).database.caCertificateId;
async function selectSection(id){await goto(settingsHref(id),{replace:true,reset:false});window.scrollTo({top:0});}
function settingsKey(event,index){if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End'].includes(event.key))return;event.preventDefault();const next=event.key==='Home'?0:event.key==='End'?sections.length-1:(index+(['ArrowRight','ArrowDown'].includes(event.key)?1:-1)+sections.length)%sections.length;selectSection(sections[next][0]);event.currentTarget.parentElement.querySelectorAll('button')[next].focus();}
async function selectEnvironment(environment){environmentDrafts[settings.environment]=structuredClone(settings);pending='environment';error='';try{settings=environmentDrafts[environment]||await api.settings(environment);databaseCheck=null;serviceChecks={};}catch(e){error=e.message;}finally{pending='';}}
function discard(){settings=JSON.parse(savedSettings);environmentDrafts={};databaseCheck=null;serviceChecks={};certificateFile=null;certificateName='';error='';notice='';}
async function saveSettings(){pending='save';error='';try{settings=await api.saveSettings(settings);savedSettings=JSON.stringify(settings);environmentDrafts={};notice='Settings saved';window.dispatchEvent(new Event('console-connections-saved'));await invalidate('console:settings').catch(()=>{});return true;}catch(e){error=e.message;await tick();errorBanner?.scrollIntoView({block:'center',behavior:'smooth'});errorBanner?.focus({preventScroll:true});return false;}finally{pending='';}}
async function testDatabase(){pending='test';databaseCheck=null;const fingerprint=databaseFingerprint;try{const result=await api.testDatabase(settings.database,settings.environment);if(databaseFingerprint===fingerprint)databaseCheck={...result,fingerprint};}catch(e){if(databaseFingerprint===fingerprint)databaseCheck={ok:false,message:e.message,fingerprint};}finally{pending='';}}
async function testService(kind,endpoint){pending=`test-${kind}`;serviceChecks={...serviceChecks,[kind]:null};try{serviceChecks={...serviceChecks,[kind]:{...await api.testService(kind,endpoint,kind==='superposition'?settings.superposition:undefined,settings.environment),endpoint}};}catch(e){serviceChecks={...serviceChecks,[kind]:{ok:false,message:e.message,endpoint}};}finally{pending='';}}
async function importCertificate(){if(!certificateFile)return;pending='certificate';error='';try{const added=await api.importCertificate(certificateFile,certificateName);certificates=[...certificates,added];settings.database.caCertificateId=added.id;settings.database.caPath='';certificateFile=null;certificateName='';}catch(e){error=e.message;}finally{pending='';}}
async function removeCertificate(id){pending='certificate';error='';try{await api.removeCertificate(id);certificates=certificates.filter(item=>item.id!==id);}catch(e){error=e.message;}finally{pending='';}}
async function deleteWorker(id){pending='worker';error='';try{await api.removeWorker(id);workers=workers.filter(worker=>worker.id!==id);}catch(e){error=e.message;}finally{pending='';}}
function workerCommand(){return `LOADTEST_CONTROL_PLANE_URL=${window.location.origin} LOADTEST_WORKER_NAME=worker-1 LOADTEST_WORKER_ENROLLMENT_TOKEN='<server enrollment token>' LOADTEST_WORKER_TPS=250 LOADTEST_WORKER_VUS=2500 npm run worker:agent`;}
async function copyWorker(){try{await navigator.clipboard.writeText(workerCommand());copied=true;setTimeout(()=>copied=false,2000);}catch{error='Clipboard unavailable. Select and copy the command below.';}}

async function requestConfirmation(action){confirmation=action;await tick();leaveNotice?.focus({preventScroll:true});}
async function confirmChanges(){if(pending)return;if(confirmation==='save'){await saveSettings();}else if(confirmation==='discard'){discard();}}
async function continueEditing(){leaveTarget=null;confirmation='';await tick();document.getElementById(`settings-tab-${section}`)?.focus();}
async function leaveSettings(save){
 if(pending||!leaveTarget)return;
 const target=leaveTarget;
 if(save){if(!await saveSettings())return;}else discard();
 leaveTarget=null;confirmation='';allowLeave=true;
 await goto(target.url,{replace:target.replace});
}
beforeNavigate(navigation=>{
 if(allowLeave){allowLeave=false;return;}
 if(navigation.to?.url.pathname==='/settings'&&!navigation.willUnload){leaveTarget=null;confirmation='';return;}
 if(pending){navigation.cancel();return;}
 if(!dirty)return;
 navigation.cancel();
 if(navigation.willUnload||!navigation.to)return;
 leaveTarget={url:navigation.to.url,replace:navigation.type==='popstate'};confirmation='leave';
 tick().then(()=>{leaveNotice?.scrollIntoView({block:'center'});leaveNotice?.focus({preventScroll:true});});
});
onMount(()=>{
 workerInstructions=workerCommand();
 const unload=e=>{if(dirty){e.preventDefault();e.returnValue='';}};window.addEventListener('beforeunload',unload);
 const poll=setInterval(()=>{if(section==='workers')api.workers().then(x=>workers=x).catch(e=>error=e.message);},5000);
 return()=>{clearInterval(poll);window.removeEventListener('beforeunload',unload);};
});
</script>
<svelte:head><title>Settings · Testbench</title></svelte:head>
<main class="settings-page">
 <div class="section-head"><div><h1>Settings</h1><p>Configure connections and test preferences.</p></div></div>
 {#if error}<div class="message error" role="alert" tabindex="-1" bind:this={errorBanner}>{error}</div>{/if}
 <section class="settings" aria-label="Console settings" data-section={section}>
  <div class="settings-body">
   <div class="settings-tabs" role="tablist" aria-label="Settings sections" aria-orientation="vertical">{#each sections as [id,label],index}<button type="button" class="quiet" role="tab" id={`settings-tab-${id}`} aria-controls={`settings-panel-${id}`} tabindex={section===id?0:-1} onkeydown={e=>settingsKey(e,index)} aria-selected={section===id} class:active={section===id} onclick={()=>selectSection(id)}>{label}</button>{/each}</div>
   <fieldset class="settings-grid" disabled={!!pending}>
      <div id="settings-panel-environment" role="tabpanel" aria-labelledby="settings-tab-environment" hidden={section!=='environment'}>
        <h2>Environment <InfoTooltip id="environment-access-info" label="Environment access information" text={environmentAccessNote(settings.environment)}/></h2>
        <p>Select where connections, load tests and faults run.</p>
        <label>Provider<select value={settings.environment} disabled={!!pending} onchange={e=>selectEnvironment(e.currentTarget.value)}><option value="unassigned" disabled>Choose an environment</option>{#each environments as environment}<option value={environment.id}>{environment.label}</option>{/each}</select></label>
        {#if settings.environment==='gcp'}<p class="environment-note" role="note"><Icon name="info" size={16}/><span>GCP fault tests require authenticated <code>gcloud</code> access on the console host and permissions for the configured resources. Kubernetes faults also require the correct context and workload permissions.</span></p>{/if}
        {#if settings.environment==='aws'}<p class="environment-note" role="note"><Icon name="info" size={16}/><span>AWS faults require authenticated <code>aws</code> access on the console host. RDS failover requires Multi-AZ; ElastiCache needs automatic failover and a replica.</span></p>{/if}
      </div>
      {#if section==='grafana'}<div id="settings-panel-grafana" role="tabpanel" aria-labelledby="settings-tab-grafana"><GrafanaSettings bind:value={settings.grafana} environment={settings.environment} disabled={!!pending}/></div>{/if}
      <div id="settings-panel-connections" role="tabpanel" aria-labelledby="settings-tab-connections" hidden={section!=='connections'}>
        <h2>Services</h2>
        <div class="fields"><div class="service-field"><label for="service-router">Router</label><div class="service-input"><input id="service-router" bind:value={settings.router}/><button type="button" class="secondary" class:check-success={serviceChecks.router?.endpoint===settings.router&&serviceChecks.router?.ok} class:check-failed={serviceChecks.router?.endpoint===settings.router&&serviceChecks.router&&!serviceChecks.router.ok} disabled={!!pending||!environments.find(e=>e.id===settings.environment)?.supported} onclick={()=>testService('router',settings.router)}>{pending==='test-router'?'Checking…':serviceChecks.router?.endpoint===settings.router?serviceChecks.router.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.router?.endpoint===settings.router}<small class:invalid={!serviceChecks.router.ok} role="status">{serviceChecks.router.message}</small>{/if}</div><div class="service-field"><label for="service-modularPm">Payment methods</label><div class="service-input"><input id="service-modularPm" bind:value={settings.modularPm}/><button type="button" class="secondary" class:check-success={serviceChecks.modularPm?.endpoint===settings.modularPm&&serviceChecks.modularPm?.ok} class:check-failed={serviceChecks.modularPm?.endpoint===settings.modularPm&&serviceChecks.modularPm&&!serviceChecks.modularPm.ok} disabled={!!pending||!environments.find(e=>e.id===settings.environment)?.supported} onclick={()=>testService('modularPm',settings.modularPm)}>{pending==='test-modularPm'?'Checking…':serviceChecks.modularPm?.endpoint===settings.modularPm?serviceChecks.modularPm.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.modularPm?.endpoint===settings.modularPm}<small class:invalid={!serviceChecks.modularPm.ok} role="status">{serviceChecks.modularPm.message}</small>{/if}</div><label>Router admin key<input type="password" bind:value={settings.adminApiKey} placeholder={settings.hasAdminApiKey ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Connector<input bind:value={settings.connector} /></label></div>
        <h2>Superposition</h2>
        <div class="fields"><div class="service-field"><label for="service-superposition">Endpoint</label><div class="service-input"><input id="service-superposition" bind:value={settings.superposition.endpoint}/><button type="button" class="secondary" class:check-success={serviceChecks.superposition?.endpoint===settings.superposition.endpoint&&serviceChecks.superposition?.ok} class:check-failed={serviceChecks.superposition?.endpoint===settings.superposition.endpoint&&serviceChecks.superposition&&!serviceChecks.superposition.ok} disabled={!!pending||!environments.find(e=>e.id===settings.environment)?.supported} onclick={()=>testService('superposition',settings.superposition.endpoint)}>{pending==='test-superposition'?'Checking…':serviceChecks.superposition?.endpoint===settings.superposition.endpoint?serviceChecks.superposition.ok?'Succeeded':'Failed':'Test'}</button></div>{#if serviceChecks.superposition?.endpoint===settings.superposition.endpoint}<small class:invalid={!serviceChecks.superposition.ok} role="status">{serviceChecks.superposition.message}</small>{/if}</div><label>Authentication<select bind:value={settings.superposition.accessHeader}><option value="x-superposition-access">x-superposition-access header</option><option value="authorization">Authorization bearer token</option></select></label><label>Token<input type="password" bind:value={settings.superposition.token} placeholder={settings.superposition.hasToken ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Organization<input bind:value={settings.superposition.organizationId} /></label><label>Workspace<input bind:value={settings.superposition.workspaceId} /></label><label>Refresh timeout · seconds<input type="number" min="0" bind:value={settings.superposition.refreshSeconds}/></label><label>Poll every · seconds<input type="number" min="0.25" max="60" step="0.25" bind:value={settings.superposition.pollSeconds}/></label></div>
        <label>Defaults<textarea rows="5" bind:value={settings.superposition.defaultsJson}></textarea></label>
      </div><div id="settings-panel-load" role="tabpanel" aria-labelledby="settings-tab-load" hidden={section!=='load'}>
        <h2>Load generator</h2>
        <p>VU capacity applied when each load or smoke run starts.</p>
        <div class="fields"><label>Preallocated VUs per TPS<input type="number" min="0.1" step="0.1" bind:value={settings.preAllocatedVusMultiplier}/></label><label>Maximum VUs per TPS<input type="number" min="0.1" step="0.1" bind:value={settings.maxVusMultiplier}/></label></div>
      </div><div id="settings-panel-results" role="tabpanel" aria-labelledby="settings-tab-results" hidden={section!=='results'}>
        <h2>Results limits</h2>
        <p>Applied to future runs. Each run keeps its own copy of these limits.</p>
        <div class="fields">{#each resultsLimitFields as [key,label,unit,max]}<label for={`results-${key}`}>{label} · {unit}<input id={`results-${key}`} type="number" min={key==='recoveryWindowSeconds'?0.1:0} {max} step={key==='maximumLostPayments'?1:'any'} bind:value={settings.resultsLimits[key]} required/></label>{/each}</div>
        <button type="button" class="secondary" onclick={()=>settings.resultsLimits={...defaultResultsLimits}}>Reset to defaults</button>
      </div><div id="settings-panel-workers" role="tabpanel" aria-labelledby="settings-tab-workers" hidden={section!=='workers'}>
        <h2>Workers</h2><p>Load agents connect outbound and report their available TPS and VU capacity.</p>
        {#if !settings.workerEnrollmentConfigured}<div class="message error" role="alert">Worker enrollment is disabled. Set <code>LOADTEST_WORKER_ENROLLMENT_TOKEN</code> in <code>loadtest/console/.env</code>, then restart the console.</div>{/if}
        <details class="advanced"><summary>Connect a worker</summary><p>Run from loadtest/console on each load machine. Use a unique name and the enrollment token configured on the master.</p><pre class="onboarding">{workerInstructions}</pre><button type="button" class="secondary" onclick={copyWorker}>{copied?'Copied':'Copy command'}</button><p>Fault commands run on the master. Workers generate load only. Health refreshes every 5 seconds.</p></details><div class="worker-list">{#each workers as worker}<div><span class:offline={worker.status==='offline'}></span><strong>{worker.name}</strong><small>{worker.status} · {worker.capacityTps} TPS · {worker.capacityVus} VUs</small>{#if worker.id!=='local'}<button class="icon-button danger" aria-label={`Remove ${worker.name}`} disabled={worker.status!=='offline'||!!pending} onclick={()=>deleteWorker(worker.id)}><Icon name="close"/></button>{/if}</div>{:else}<p>No workers registered.</p>{/each}</div>
      </div>
      <div id="settings-panel-database" role="tabpanel" aria-labelledby="settings-tab-database" hidden={section!=='database'}>
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
          <label>TLS server name (optional)<input bind:value={settings.database.serverName} placeholder="Database hostname when using an SSH tunnel" /></label>
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

   </fieldset>
  </div>
  <div class="actions settings-save" bind:this={leaveNotice} role="region" aria-label="Settings actions" aria-labelledby={confirmation?'settings-confirm-title':undefined} tabindex="-1">
   {#if confirmation}
    <div class="confirmation-copy"><h2 id="settings-confirm-title">{confirmation==='leave'?'Save before leaving Settings?':confirmation==='save'?'Save these changes?':'Discard unsaved changes?'}</h2><small>{confirmation==='discard'?'Your saved settings will be kept.':'Changes apply to this environment.'}</small></div>
    <button type="button" class="quiet" disabled={!!pending} onclick={continueEditing}>{confirmation==='leave'?'Continue editing':'Cancel'}</button>
    {#if confirmation==='leave'}<button type="button" class="secondary danger" disabled={!!pending} onclick={()=>leaveSettings(false)}>Discard and leave</button><button type="button" disabled={!!pending} onclick={()=>leaveSettings(true)}>{pending==='save'?'Saving…':'Save and leave'}</button>
    {:else}<button type="button" class:danger={confirmation==='discard'} disabled={!!pending} onclick={confirmChanges}>{pending==='save'?'Saving…':confirmation==='save'?'Save changes':'Discard changes'}</button>{/if}
   {:else}
    <small class:saved={!!notice} role="status">{dirty?'Unsaved changes':notice||'All changes saved'}</small><button type="button" class="secondary" disabled={!!pending||!dirty} onclick={()=>requestConfirmation('discard')}>Discard changes</button><button type="button" disabled={!!pending||!dirty} onclick={()=>requestConfirmation('save')}>Save settings</button>
   {/if}
  </div>
 </section>
</main>

<style>
.settings-page{display:flex;flex-direction:column;min-height:calc(100dvh - var(--header-height) - var(--nav-height));padding-bottom:0}.confirmation-copy{margin-right:auto;min-width:0}.confirmation-copy h2{font-size:13px;margin:0 0 4px}.confirmation-copy small{display:block}.settings-save:focus{outline:none}@media(max-width:760px){.confirmation-copy{flex-basis:100%}}
.environment-note{display:flex;align-items:flex-start;gap:10px;margin:16px 0 0;padding:12px 14px;border:1px solid var(--border);border-radius:6px;background:var(--accent-soft);color:var(--ink);font-size:12px;line-height:1.6}.environment-note :global(svg){flex-shrink:0;margin-top:2px;color:var(--accent)}

.custom-ca{border-top:1px solid var(--border);padding-top:14px}.custom-ca summary{flex-wrap:wrap}.custom-ca summary>span{margin-left:auto;font-size:12px;font-weight:400;color:var(--muted)}.ca-options{display:grid;gap:14px;padding-top:16px}.ca-options>button{justify-self:start}.ca-options .certificate-list{margin-top:0}
.service-input{display:flex;align-items:center;gap:8px;margin-top:6px}.service-input input{margin:0;min-width:0}.service-input button{flex-shrink:0}.database-test{margin-top:16px}.service-field small{display:block;margin-top:6px;font-size:11px}

.settings{padding:0;overflow:visible;display:flex;flex:1;flex-direction:column}.settings-body{display:grid;flex:1;grid-template-columns:160px minmax(0,1fr);gap:24px;padding:0 0 24px}.settings-tabs{display:flex;flex-direction:column;gap:5px;position:sticky;top:calc(var(--header-height) + var(--nav-height) + 24px);align-self:start}.settings-tabs button{justify-content:flex-start;text-align:left;white-space:normal;font-size:12px;padding:10px 12px}.settings-tabs button.active{--button-bg:var(--accent-soft);--button-fg:var(--accent);--button-hover-bg:var(--selected-hover);--button-hover-fg:var(--accent)}.settings-grid{min-width:0;border:0;border-left:1px solid var(--border);margin:0;padding:0 0 0 24px}.settings-grid :global(h2){font-size:14px;margin:0 0 18px}.settings-grid :global(.fields+h2){margin-top:28px}.settings-grid>div>.fields{margin-bottom:20px}.settings-grid p{font-size:12px;color:var(--muted)}.settings-grid>div>label{margin:18px 0}.settings-save{flex-shrink:0;position:sticky;bottom:0;z-index:10;background:var(--surface-muted);padding:16px 0;margin:0;border-top:1px solid var(--border)}.settings-save small{margin-right:auto;color:var(--muted);font-size:12px}.settings-save small.saved{color:var(--success)}.onboarding{white-space:pre-wrap;overflow-wrap:anywhere;font-size:12px;background:var(--surface-muted);padding:14px;border-radius:8px}[hidden]{display:none!important}.certificate-list,.worker-list{display:grid;gap:8px;margin-top:12px}.certificate-list>div,.worker-list>div{display:flex;align-items:center;gap:10px;border:1px solid var(--border);border-radius:7px;padding:10px 12px}.certificate-list>div{justify-content:space-between}.certificate-list span{display:grid;gap:3px;min-width:0}.certificate-list small{overflow-wrap:anywhere}.worker-list span{width:7px;height:7px;border-radius:50%;background:var(--success);flex-shrink:0}.worker-list span.offline{background:var(--subtle)}.worker-list small{margin-left:auto;color:var(--muted)}
@media(max-width:760px){.settings-body{grid-template-columns:1fr;gap:20px;padding:0 0 20px}.settings-tabs{position:static;flex-direction:row;flex-wrap:wrap;gap:4px;padding-bottom:16px;border-bottom:1px solid var(--border)}.settings-tabs button{padding:8px 10px}.settings-grid{border:0;padding:0}.settings-save{padding:16px 0}.settings-save small{flex-basis:100%}.worker-list>div{flex-wrap:wrap}}
</style>
