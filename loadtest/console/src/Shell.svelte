<script>
import {onMount} from 'svelte';
import {page} from '$app/stores';
import {api} from './api.js';
export let data;
let settings=structuredClone(data.settings),savedSettings=JSON.stringify(settings),settingsDialog,pending='',error='',databaseCheck=null;
$: dirty=JSON.stringify(settings)!==savedSettings;
async function openSettings(){error='';databaseCheck=null;try{settings=await api.settings();savedSettings=JSON.stringify(settings);settingsDialog.showModal();}catch(e){error=e.message;}}
async function saveSettings(){pending='save';error='';try{settings=await api.saveSettings(settings);savedSettings=JSON.stringify(settings);settingsDialog.close();window.dispatchEvent(new Event('console-connections-saved'));}catch(e){error=e.message;}finally{pending='';}}
async function testDatabase(){pending='test';databaseCheck=null;try{databaseCheck=await api.testDatabase(settings.database);}catch(e){databaseCheck={ok:false,message:e.message};}finally{pending='';}}
onMount(()=>{const open=()=>openSettings();window.addEventListener('console-settings',open);return()=>window.removeEventListener('console-settings',open);});
</script>
<header>
  <a class="brand" href="/scenarios"><img src="/hyperswitch-logo.svg" alt="Hyperswitch" /><span>Load testing</span></a>
  <div class="header-actions"><button class="header-button" onclick={openSettings}><span aria-hidden="true">⚙</span> Settings</button><form action="/logout" method="POST"><button class="header-button">Sign out <span aria-hidden="true">↗</span></button></form></div>
</header>

<dialog class="settings-dialog" bind:this={settingsDialog} aria-labelledby="settings-title">
  <section class="settings">
    <div class="section-head"><div><h1 id="settings-title">Settings</h1><p>Connections and infrastructure</p></div><button class="icon-button" aria-label="Close settings" onclick={() => settingsDialog.close()}>×</button></div>
    {#if error}<div class="message error" role="alert">{error}</div>{/if}
    <div class="settings-grid">
      <div>
        <h2>Services</h2>
        <div class="fields"><label>Router<input bind:value={settings.router} /></label><label>Payment methods<input bind:value={settings.modularPm} /></label><label>Router admin key<input type="password" bind:value={settings.adminApiKey} placeholder={settings.hasAdminApiKey ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Connector<input bind:value={settings.connector} /></label></div>
        <h2>Superposition</h2>
        <div class="fields"><label>Endpoint<input bind:value={settings.superposition.endpoint} /></label><label>Token<input type="password" bind:value={settings.superposition.token} placeholder={settings.superposition.hasToken ? 'Configured · leave blank to keep' : 'Required'} /></label><label>Organization<input bind:value={settings.superposition.organizationId} /></label><label>Workspace<input bind:value={settings.superposition.workspaceId} /></label></div>
        <label>Defaults<textarea rows="5" bind:value={settings.superposition.defaultsJson}></textarea></label>
      </div>
      <div>
        <h2>Grafana</h2><label>Base URL<input bind:value={settings.grafana.baseUrl} placeholder="http://localhost:3002" /></label>
        <p>{settings.grafana.hasIdentity ? 'Server credentials configured' : 'Set GRAFANA_TOKEN in the server .env to a Viewer service-account token, then restart the console.'}</p>
        <h2 id="database-settings">Database</h2><p>Use an existing PostgreSQL account with read access to payment_intent.</p>
        <div class="fields"><label>Host<input bind:value={settings.database.host} /></label><label>Port<input type="number" bind:value={settings.database.port} /></label><label>Database<input bind:value={settings.database.database} /></label><label>Read-only user<input bind:value={settings.database.user} /></label><label>Password<input type="password" bind:value={settings.database.password} placeholder={settings.database.hasPassword ? 'Configured · leave blank to keep' : 'Required'} /></label></div>
        <label class="checkbox"><input type="checkbox" bind:checked={settings.database.ssl}/> Require verified TLS</label>
        <button class="secondary" disabled={!!pending} onclick={testDatabase}>{pending==='test'?'Checking…':'Test connection'}</button>
        {#if databaseCheck}<p role="status" class:invalid={!databaseCheck.ok}>{databaseCheck.message}</p>{/if}
      </div>
    </div>
    <div class="actions"><button class="secondary" onclick={() => settingsDialog.close()}>Close</button><button disabled={!!pending || !dirty} onclick={saveSettings}>{pending === 'save' ? 'Saving…' : 'Save settings'}</button></div>
  </section>
</dialog>


<nav aria-label="Console sections">{#each ['scenarios','fixtures','performance','results'] as item}<a class:active={$page.url.pathname===`/${item}`} href={`/${item}`}>{item[0].toUpperCase()+item.slice(1)}</a>{/each}</nav>
<slot/>
<footer>Router · {settings.router} / PM · {settings.modularPm}</footer>
