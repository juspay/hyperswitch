<script>
import Icon from './Icon.svelte';
import {api} from './api.js';import {invalidateAll} from '$app/navigation';import {setQuery} from './navigation.mjs';
import {page} from '$app/stores';import ProfileEditor from './ProfileEditor.svelte';import {flowLabel} from './flow-catalog.mjs';
export let data;
let selectedId=data.profileId,search='',error='',deleteDialog,pendingDelete,deleting=false;
$: search=$page.url.searchParams.get('q')||'';
$: visible=profiles.filter(p=>p.name.toLowerCase().includes(search.toLowerCase()));
$: profiles=data.profiles.filter(profile=>!profile.archived);
$: view=$page.url.searchParams.get('view');
$: source=profiles.find(p=>p.id===$page.url.searchParams.get('profile'));
$: copying=source||profiles.find(p=>p.id===selectedId)||profiles[0]||data.profileTemplate;
$: newScenarioHref=copying?.id?`/scenarios?view=new&profile=${encodeURIComponent(copying.id)}`:'/scenarios?view=new';
const faultCount=profile=>profile.load?.stages?.filter(stage=>stage.fault).length||0;
const merchantLabel=profile=>profile.mode==='single'?'Single merchant':`Multiple · ${profile.merchantCount.toLocaleString()} merchants`;
const readyFixture=profile=>data.fixtures.find(set=>set.status==='ready'&&set.compatible&&set.mode===profile.mode&&set.merchant_count===profile.merchantCount);
function confirmDelete(profile){pendingDelete=profile;deleteDialog.showModal();}
async function removeScenario(){if(!pendingDelete||deleting)return;deleting=true;error='';try{await api.deleteProfile(pendingDelete.id);deleteDialog.close();pendingDelete=null;await invalidateAll();}catch(e){error=e.message;}finally{deleting=false;}}
</script>
<svelte:head><title>Scenarios · Testbench</title></svelte:head>
<main>
{#if view==='new'||view==='edit'&&source}
{#key `${view}:${copying.id}`}<ProfileEditor profile={view==='new'?copying:source} creating={view==='new'}/>{/key}
{:else}
<div class="section-head"><div><h1>Scenarios</h1><p>Build traffic patterns and resilience tests.</p></div><a class="button" href={newScenarioHref}><Icon name="plus"/> New scenario</a></div>
{#if view==='edit'&&!source}<div class="message error">Scenario not found. Choose a saved scenario below.</div>{/if}
<div class="library-filters"><div class="library-search"><Icon name="search"/><input type="search" aria-label="Search scenarios" placeholder="Search scenarios" bind:value={search} oninput={()=>setQuery({q:search})}/></div></div>{#if error}<p class="invalid" role="alert">{error}</p>{/if}<div class="profile-library">
{#each visible as profile}
{@const fixture=readyFixture(profile)}
<article class="profile-card"><div class="profile-copy"><h2><a href={`/scenarios?view=edit&profile=${encodeURIComponent(profile.id)}`}>{profile.name}</a></h2><div class="profile-metadata" aria-label="Scenario summary"><span>{merchantLabel(profile)}</span><span>{profile.loadShape==='staged'?'Ramping':'Constant load'}</span><span><strong>{profile.totalRps.toLocaleString()}</strong> {profile.loadShape==='staged'?'peak TPS':'TPS'}</span><span><strong>{profile.durationSeconds.toLocaleString()}</strong> sec</span>{#if faultCount(profile)>0}<span class="faults"><strong>{faultCount(profile)}</strong> {faultCount(profile)===1?'fault':'faults'}</span>{/if}</div><div class="flow-summary">{#each profile.scenarios as flow,index}<span>{index?' · ':''}{flowLabel(flow.scenario)} {flow.weight}%</span>{/each}</div></div><div class="profile-actions"><a class="button quiet" href={`/scenarios?view=edit&profile=${encodeURIComponent(profile.id)}`} onclick={()=>selectedId=profile.id}>Edit</a><button type="button" class="quiet delete-action" onclick={()=>confirmDelete(profile)}>Delete</button><a class="button secondary" href={fixture?`/performance?profile=${encodeURIComponent(profile.id)}&fixtures=${encodeURIComponent(fixture.id)}`:`/fixtures?profile=${encodeURIComponent(profile.id)}`}>{fixture?'Test performance':'Create fixtures'}</a></div></article>
{/each}{#if !visible.length}<div class="empty-library"><h2>{profiles.length?'No matching scenarios':'No scenarios yet'}</h2><p class="muted">{profiles.length?'Try a different search.':'Create a scenario to configure traffic, merchants, and faults.'}</p>{#if !profiles.length}<a class="button secondary" href={newScenarioHref}><Icon name="plus"/> New scenario</a>{/if}</div>{/if}
</div>
{/if}
</main>
<dialog bind:this={deleteDialog} oncancel={event=>{if(deleting)event.preventDefault();}} aria-labelledby="delete-scenario-title"><h2 id="delete-scenario-title">Delete scenario?</h2><p><strong>{pendingDelete?.name}</strong> will be deleted. Existing load-test runs keep their saved snapshots.</p><div class="actions"><button type="button" class="secondary" disabled={deleting} onclick={()=>deleteDialog.close()}>Cancel</button><button type="button" class="danger" disabled={deleting} onclick={removeScenario}>{deleting?'Deleting…':'Delete scenario'}</button></div></dialog>
<style>
.library-filters{display:flex;align-items:center;gap:24px;margin:0 0 20px}.library-search{position:relative;flex:1;max-width:360px}.library-search :global(.ui-icon){position:absolute;left:12px;top:11px;color:var(--subtle)}.library-search input{margin:0;padding-left:36px;background:var(--surface)}.profile-library{background:var(--surface);border:1px solid var(--border);border-radius:var(--radius);box-shadow:var(--shadow)}.profile-card{display:flex;align-items:center;justify-content:space-between;gap:24px;padding:20px 22px;border-bottom:1px solid var(--border)}.profile-card:last-child{border-bottom:0}.profile-copy{min-width:0}.profile-card h2{font-size:15px;margin:0 0 9px}.profile-card h2 a{color:var(--ink);text-decoration:none}.profile-card h2 a:hover{color:var(--accent)}.profile-metadata{display:flex;align-items:center;gap:6px;flex-wrap:wrap;margin-bottom:8px}.profile-metadata span{display:inline-flex;align-items:center;min-height:24px;padding:4px 8px;border-radius:5px;background:var(--surface-muted);color:#536176;font-size:11px;line-height:1.2}.profile-metadata strong{font-weight:600;color:var(--ink);margin-right:3px}.profile-metadata .faults{background:var(--warning-soft);color:var(--warning)}.profile-metadata .faults strong{color:inherit}.flow-summary{color:var(--muted);font-size:11px;line-height:1.6}.profile-actions{display:flex;align-items:center;gap:6px;flex-shrink:0}.profile-actions a,.profile-actions button{font-size:12px}.profile-actions .delete-action{--button-fg:var(--danger);--button-hover-bg:var(--danger-soft);--button-hover-fg:var(--danger)}.empty-library{padding:44px 24px;text-align:center}.empty-library h2{margin-bottom:4px}.empty-library p{margin:0 0 16px}.section-head>div{min-width:0}@media(max-width:750px){.profile-card{align-items:flex-start;flex-direction:column;padding:18px;gap:16px}.profile-actions{width:100%;flex-wrap:wrap}.profile-actions a:last-child{margin-left:auto}.library-filters{gap:16px}.library-search{max-width:none}}@media(max-width:420px){.library-filters{flex-wrap:wrap}.library-search{flex-basis:100%}.profile-actions a:last-child{width:100%;margin-left:0}}
</style>
