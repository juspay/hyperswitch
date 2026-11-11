<script>
import GrafanaMetrics from './GrafanaMetrics.svelte';
export let value='native';
export let run;
export let grafana;
</script>
<div class="metrics-switch" class:grafana-selected={value==='grafana'} role="group" aria-label="Metrics source">
  <button type="button" class:active={value==='native'} aria-pressed={value==='native'} onclick={()=>value='native'}>Native</button>
  <button type="button" class:active={value==='grafana'} aria-pressed={value==='grafana'} onclick={()=>value='grafana'}>Grafana</button>
</div>
{#if value==='native'}<slot/>{:else}<GrafanaMetrics {run} {grafana}/>{/if}
<style>
.metrics-switch{position:relative;display:grid;grid-template-columns:repeat(2,1fr);width:max-content;margin:0 0 16px auto;padding:3px;border-radius:9px;background:var(--surface-muted);box-shadow:inset 0 0 0 1px var(--border);gap:2px;isolation:isolate}
.metrics-switch::before{content:'';position:absolute;z-index:-1;top:3px;bottom:3px;left:3px;width:calc((100% - 8px)/2);border-radius:6px;background:var(--surface);box-shadow:inset 0 0 0 1px var(--border-strong),0 1px 2px #18263b08;transition:transform 220ms cubic-bezier(.2,.8,.2,1)}
.metrics-switch.grafana-selected::before{transform:translateX(calc(100% + 2px))}
.metrics-switch button{--button-bg:transparent;--button-fg:var(--muted);--button-border:transparent;--button-hover-bg:transparent;--button-hover-fg:var(--ink);--button-hover-border:transparent;min-width:82px;min-height:30px;padding:6px 16px;font-size:12px;font-weight:550;border-radius:6px;transition:color 160ms ease}
.metrics-switch button.active{--button-fg:var(--accent);--button-hover-fg:var(--accent-hover)}
@media(prefers-reduced-motion:reduce){.metrics-switch::before,.metrics-switch button{transition:none}}
</style>
