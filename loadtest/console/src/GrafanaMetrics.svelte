<script>
import GrafanaPanel from './GrafanaPanel.svelte';
import {configuredGrafanaPanels} from './grafana-panels.mjs';
export let run;
export let grafana;
$: panels=configuredGrafanaPanels(run,grafana?.panels);
</script>
{#if !panels.length&&run?.startedAt}<p class="metrics-note">No panels configured. <button type="button" class="quiet" onclick={()=>window.dispatchEvent(new CustomEvent('console-settings',{detail:{section:'grafana'}}))}>Add Grafana panels</button></p>{/if}
{#if panels.length}
<section class="grafana-metrics" aria-label="Grafana metrics">
    {#if grafana?.hasIdentity}
      <div class="grafana-grid">{#each panels as panel (panel.url)}<GrafanaPanel {panel} url={panel.url} revision={grafana.revision||''}/>{/each}</div>
    {:else}
      <p class="metrics-note">Configure Grafana access in the console to load embedded panels.</p>
    {/if}
</section>
{/if}
<style>
.grafana-metrics{margin-bottom:16px}.metrics-note{margin:0 0 14px;color:var(--muted);font-size:12px}.grafana-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:14px}
@media(max-width:1000px){.grafana-grid{grid-template-columns:1fr}}
</style>
