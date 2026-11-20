<script>
import {connectorCatalog,defaultConnectorPlan} from './connector-config.mjs';
export let plan=defaultConnectorPlan();export let value={};
function update(connector,field,text){value={...value,[connector]:{...value[connector],credentials:{...value[connector]?.credentials,[field]:text}}};}
function metadata(connector,text){value={...value,[connector]:{...value[connector],metadataText:text}};}
</script>
{#if plan.mode==='real'||plan.mix.length>1}
<section class="connector-credentials"><h2>Connector credentials</h2><p>Sandbox accounts for each merchant in this fixture set.</p>
{#each plan.mix as row (row.connector)}{@const definition=connectorCatalog.find(c=>c.id===row.connector)}<fieldset><legend>{definition?.label||row.connector}</legend><div class="fields">{#each definition?.fields||[] as field}<label>{field.label}<input type="password" autocomplete="new-password" value={value[row.connector]?.credentials?.[field.name]||''} oninput={e=>update(row.connector,field.name,e.currentTarget.value)}/></label>{/each}</div><details><summary>Account metadata</summary><label>Connector-specific metadata · JSON<textarea rows="3" value={value[row.connector]?.metadataText||''} oninput={e=>metadata(row.connector,e.currentTarget.value)} placeholder={'{}'}></textarea></label></details></fieldset>{/each}
</section>
{/if}
<style>.connector-credentials{grid-column:1/-1;border-top:1px solid var(--border);padding-top:16px}.connector-credentials p{font-size:12px;color:var(--muted)}fieldset{border:1px solid var(--border);border-radius:7px;padding:16px;margin:12px 0}legend{font-size:13px;font-weight:550}details{margin-top:12px}textarea{font-family:monospace}</style>
