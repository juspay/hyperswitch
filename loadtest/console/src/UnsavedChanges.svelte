<script>
export let save=async()=>false;
let dialog,resolve,pending=false,error='';
export function ask(){error='';dialog.showModal();return new Promise(done=>resolve=done);}
function answer(value){dialog.close();resolve?.(value);resolve=null;}
async function saveAndLeave(){pending=true;error='';try{if(await save())answer(true);else error='Could not save. Review the highlighted fields.';}catch(e){error=e.message;}finally{pending=false;}}
</script>
<dialog class="unsaved-dialog" bind:this={dialog} aria-label="Unsaved changes" oncancel={e=>{e.preventDefault();if(!pending)answer(false);}}>
<div class="unsaved-copy"><h2>Save your changes?</h2><p>Your edits haven’t been saved yet.</p></div>
{#if error}<p class="invalid" role="alert">{error}</p>{/if}
<div class="actions"><button type="button" class="quiet" disabled={pending} onclick={()=>answer(false)}>Continue editing</button><button type="button" class="secondary danger" disabled={pending} onclick={()=>answer(true)}>Discard</button><button type="button" disabled={pending} onclick={saveAndLeave}>{pending?'Saving…':'Save changes'}</button></div>
</dialog>
<style>.unsaved-dialog{width:min(520px,calc(100vw - 32px));padding:24px}.unsaved-copy h2{font-size:20px;margin:0 0 8px;letter-spacing:-.3px}.unsaved-copy p{margin:0;color:var(--muted);font-size:13px}.actions{gap:8px;flex-wrap:wrap;justify-content:flex-end}.actions .quiet{margin-right:auto}@media(max-width:460px){.actions .quiet{width:100%;order:3;margin:4px 0 0}.actions>button:not(.quiet){flex:1}}</style>
