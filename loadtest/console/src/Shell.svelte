<script>
import {onMount} from 'svelte';
import {afterNavigate,goto} from '$app/navigation';
import Icon from './Icon.svelte';
import Brand from './Brand.svelte';
import ServiceIndicators from './ServiceIndicators.svelte';
import {environmentLabel} from './environments.mjs';
import {page,navigating} from './app-state.mjs';
import {navigationContext,rememberPage,sectionHref} from './navigation.mjs';
import {settingsHref,settingsSection} from './settings-sections.mjs';
export let data;
const sections=['scenarios','fixtures','performance','results','settings'];
$: loadingSection=$navigating?.to?.url.pathname!==$page.url.pathname&&sections.includes($navigating?.to?.url.pathname?.slice(1))?$navigating.to.url.pathname.slice(1):'';
function openSettings(event){goto(settingsHref(settingsSection(event?.detail?.section)));}
afterNavigate(({to})=>{if(to)rememberPage(new URL(window.location.href));});
onMount(()=>{window.addEventListener('console-settings',openSettings);return()=>window.removeEventListener('console-settings',openSettings);});
</script>
<header class="app-header">
 <div class="brand-group"><a class="brand" href="/scenarios" aria-label="JUSPAY Hyperswitch · Testbench home"><img class="company-logo" src="/hyperswitch-logo.svg" alt="JUSPAY Hyperswitch"/><span class="product-brand"><Brand/></span></a><button type="button" class="environment-label" aria-label={`Change environment: ${environmentLabel(data.settings.environment)}`} onclick={()=>openSettings({detail:{section:'environment'}})}>{environmentLabel(data.settings.environment)}</button></div>
 <div class="header-actions"><form action="/logout" method="POST"><button class="header-button" aria-label="Sign out" title="Sign out"><span>Sign out</span><Icon name="logout"/></button></form></div>
</header>
<div class="console-frame">
 <nav class="section-nav" aria-label="Console sections" aria-busy={!!$navigating}><div class="section-links">{#each sections as item}<a class:active={$page.url.pathname===`/${item}`||$page.url.pathname.startsWith(`/${item}/`)} class:opening={$navigating?.to?.url.pathname===`/${item}`} aria-current={$page.url.pathname===`/${item}`?'page':undefined} href={sectionHref(item,$page.url,$navigationContext)}><Icon name={item}/><span>{item[0].toUpperCase()+item.slice(1)}</span></a>{/each}</div></nav>
 <aside class="service-sidebar" aria-labelledby="service-health-heading"><div class="service-health"><h2 id="service-health-heading">HEALTH</h2><ServiceIndicators settings={data.settings}/></div></aside>
 <div class="console-content" aria-busy={!!loadingSection}><div hidden={!!loadingSection}><slot/></div>{#if loadingSection}<section class="section-loader" role="status" aria-live="polite"><span class="loading-spinner" aria-hidden="true"></span><p>Loading {loadingSection}…</p></section>{/if}</div>
</div>
<style>
.section-nav a.opening{color:var(--accent);background:var(--accent-soft)}.section-loader{min-height:calc(100dvh - var(--header-height) - var(--nav-height));display:flex;align-items:center;justify-content:center;flex-direction:column;gap:14px;padding:32px;color:var(--muted)}.section-loader p{margin:0;font-size:14px}.loading-spinner{width:28px;height:28px;border:3px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:section-spin .8s linear infinite}@keyframes section-spin{to{transform:rotate(360deg)}}@media(prefers-reduced-motion:reduce){.loading-spinner{animation:none}}
.brand-group{display:flex;align-items:center;gap:14px;flex-shrink:0}.environment-label{--button-hover-bg:var(--accent-soft);--button-hover-fg:var(--accent);--button-hover-border:var(--accent);min-height:0;font-size:11px;font-weight:400;line-height:1.2;color:var(--accent);background:var(--accent-soft);border:1px solid transparent;border-radius:5px;padding:4px 8px;white-space:nowrap}.brand{gap:14px}.company-logo{display:block;width:175px;height:auto}.product-brand{display:flex;align-items:center;padding-left:14px;border-left:1px solid var(--border)}
@media(max-width:900px){.company-logo{width:150px}.product-brand{padding-left:12px}}
@media(max-width:600px){.company-logo{width:138px}.product-brand{display:none}}
</style>
