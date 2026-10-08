<script>
import {page} from '$app/stores';import ProfileEditor from './ProfileEditor.svelte';import {profileSummary,flowLabel} from './flow-catalog.mjs';
export let data;
let selectedId=data.profileId;
$: profiles=data.profiles;
$: view=$page.url.searchParams.get('view');
$: source=profiles.find(p=>p.id===$page.url.searchParams.get('profile'));
$: copying=source||profiles.find(p=>p.id===selectedId)||profiles[0];
</script>
<svelte:head><title>Scenarios · Hyperswitch</title></svelte:head>
<main>
{#if view==='new'||view==='edit'&&source}
{#key `${view}:${copying.id}`}<ProfileEditor profile={view==='new'?copying:source} creating={view==='new'}/>{/key}
{:else}
<div class="section-head"><div><h1>Scenarios</h1><p>Saved scenarios</p></div><a class="button secondary" href={`/scenarios?view=new&profile=${encodeURIComponent(copying.id)}`}>+ New scenario</a></div>
{#if view==='edit'&&!source}<div class="message error">Scenario not found. Choose a saved scenario below.</div>{/if}
<div class="profile-library">
{#each profiles as profile}
<article class="profile-card"><div class="profile-copy"><h2>{profile.name}</h2><p>{profileSummary(profile)}{#if profile.load?.stages?.some(stage=>stage.fault)} · {profile.load.stages.filter(stage=>stage.fault).length} faults{/if}</p><div class="flow-summary">{#each profile.scenarios as flow,index}<span>{index?' · ':''}{flowLabel(flow.scenario)} {flow.weight}%</span>{/each}</div></div><div class="profile-actions"><a class="button secondary" href={`/scenarios?view=edit&profile=${encodeURIComponent(profile.id)}`} onclick={()=>selectedId=profile.id}>Edit</a><a class="button" href={`/performance?profile=${encodeURIComponent(profile.id)}`}>Open Performance →</a></div></article>
{/each}
</div>
{/if}
</main>
<style>.profile-library{display:grid;gap:14px}.profile-card{display:flex;align-items:center;justify-content:space-between;gap:24px;padding:24px;background:#fff;border:1px solid var(--border);border-radius:12px}.profile-copy{min-width:0}.profile-card h2{font-size:16px;margin:0 0 8px}.profile-card p{font-size:13px;color:#475467;margin:0 0 10px}.flow-summary{color:#98a2b3;font-size:12px;line-height:1.8}.profile-actions{display:flex;gap:10px;flex-shrink:0}.profile-actions a{font-size:12px}@media(max-width:750px){.profile-card{align-items:flex-start;flex-direction:column;padding:20px}.profile-actions{flex-wrap:wrap;width:100%}.profile-actions a{justify-content:center}.section-head{gap:16px}}
</style>
