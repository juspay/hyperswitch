<script>
import {onMount} from 'svelte';
export let panel;export let url;export let baseUrl;
let loaded=false,failed=false,attempt=0,timer,embedUrl='',mounted=false;
async function retry(){const current=++attempt;loaded=false;failed=false;embedUrl='';clearTimeout(timer);timer=setTimeout(()=>{if(!loaded)failed=true;},15000);
 try{const response=await fetch(`/api/grafana/embed?panel=${encodeURIComponent(url)}`);if(!response.ok)throw Error('Embed unavailable');const result=await response.json();if(current===attempt)embedUrl=result.url;}catch{if(current===attempt)failed=true;}}
onMount(()=>{mounted=true;return()=>{attempt++;clearTimeout(timer);};});
$: if(mounted&&url)retryFor(url);
function retryFor(_url){retry();}
</script>
<article class="grafana-panel">
{#if !loaded}<p role="status">{failed?'Panel is taking longer to load.':'Loading panel…'}{#if failed}<button class="quiet" onclick={retry}>Retry</button>{/if}</p>{/if}
{#if embedUrl}<iframe title={panel.title} src={embedUrl} sandbox="allow-scripts allow-same-origin" referrerpolicy="no-referrer" onload={()=>{loaded=true;failed=false;clearTimeout(timer);}} onerror={()=>failed=true}></iframe>{/if}
<a class="quiet panel-link" aria-label={`Open ${panel.title} in Grafana`} href={new URL(url.replace(/^\/grafana/,''),baseUrl||'http://localhost:3000').href} target="_blank" rel="noreferrer">Open ↗</a>
</article>
<style>p{font-size:12px;color:var(--muted);padding:8px}.quiet{font-size:12px}.panel-link{display:block;text-align:right;padding:8px 12px}</style>
