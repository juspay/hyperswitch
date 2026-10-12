<script>
import {onMount} from 'svelte';
export let state='connecting';export let updatedAt=0;
let now=Date.now();onMount(()=>{const timer=setInterval(()=>now=Date.now(),1000);return()=>clearInterval(timer);});
$: stale=updatedAt>0&&now-updatedAt>15000;
</script>
<span class:unhealthy={state!=='connected'||stale} class="connection" role="status">{state==='connected'&&!stale?'● Connected':state==='connecting'?'Connecting…':'Reconnecting…'}{#if updatedAt&&(stale||state!=='connected')} · last update {Math.max(0,Math.floor((now-updatedAt)/1000))}s ago{/if}</span>
<style>.connection{font-size:11px;color:#027a48}.unhealthy{color:#b7781c}</style>
