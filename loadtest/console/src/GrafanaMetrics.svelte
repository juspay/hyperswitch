<script>
import GrafanaPanel from './GrafanaPanel.svelte';
import {unifiedPerformancePanels} from './grafana-panels.mjs';
export let run;
export let grafana;
$: panels=unifiedPerformancePanels(run);
</script>
{#if panels.length}
<section class="grafana-metrics" aria-label="Grafana metrics">
    {#if grafana?.hasIdentity}
      <div class="grafana-grid">{#each panels as panel (panel.title)}<GrafanaPanel {panel} url={panel.url} baseUrl={grafana.baseUrl}/>{/each}</div>
    {:else}
      <p class="metrics-note">Configure Grafana access in the console to load embedded panels.</p>
    {/if}
</section>
{/if}
<style>
.grafana-metrics{margin-bottom:16px}.metrics-note{margin:0 0 14px;color:var(--muted);font-size:12px}.grafana-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:14px}
@media(max-width:1000px){.grafana-grid{grid-template-columns:1fr}}
</style>
