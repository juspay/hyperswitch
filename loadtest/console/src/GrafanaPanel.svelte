<script>
import {onMount} from 'svelte';
export let panel;export let url;export let baseUrl;
let loaded=false,failed=false,attempt=0,timer;
function retry(){loaded=false;failed=false;attempt++;clearTimeout(timer);timer=setTimeout(()=>{if(!loaded)failed=true;},15000);}
onMount(()=>{retry();return()=>clearTimeout(timer);});
</script>
<article class="grafana-panel">
{#if !loaded}<p role="status">{failed?'Panel is taking longer to load.':'Loading panel…'}{#if failed}<button class="quiet" onclick={retry}>Retry</button>{/if}</p>{/if}
{#key url+attempt}<iframe title={panel.title} src={url} onload={()=>{loaded=true;failed=false;clearTimeout(timer);}} onerror={()=>failed=true}></iframe>{/key}
<a class="quiet panel-link" aria-label={`Open ${panel.title} in Grafana`} href={new URL(url.replace(/^\/grafana/,''),baseUrl||'http://localhost:3000').href} target="_blank" rel="noreferrer">Open ↗</a>
</article>
<style>p{font-size:12px;color:var(--muted);padding:8px}.quiet{font-size:12px}.panel-link{display:block;text-align:right;padding:8px 12px}</style>
