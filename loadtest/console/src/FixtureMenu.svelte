<script>
 import {tick,onMount} from 'svelte';import Icon from './Icon.svelte';
 export let id;export let href='';export let disabled=false;export let deleting=false;export let ondelete=()=>{};
 let open=false,confirm=false,root,trigger,panel,left=0,top=0;
 function close(){open=false;confirm=false;}
 async function toggle(){if(open){close();return;}open=true;await tick();const rect=trigger.getBoundingClientRect(),height=panel.offsetHeight;left=Math.max(12,rect.right-180);top=rect.bottom+6;if(top+height>window.innerHeight-12)top=Math.max(12,rect.top-height-6);panel.querySelector('a,button')?.focus();}
 function keyboard(event){if(!open)return;if(event.key==='Escape'){event.preventDefault();close();trigger.focus();}else if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)){event.preventDefault();const items=[...panel.querySelectorAll('a,button:not(:disabled)')],index=items.indexOf(document.activeElement);items[event.key==='Home'?0:event.key==='End'?items.length-1:(index+(event.key==='ArrowDown'?1:-1)+items.length)%items.length]?.focus();}else if(event.key==='Tab')close();}
 onMount(()=>{document.addEventListener('scroll',close,true);return()=>document.removeEventListener('scroll',close,true);});
</script>
<svelte:window onpointerdown={event=>{if(open&&!root?.contains(event.target))close();}} onkeydown={keyboard} onresize={close}/>
<div class="fixture-menu" bind:this={root}>
 <button bind:this={trigger} type="button" class="quiet menu-trigger" aria-label={`Fixture actions ${id.slice(0,8)}`} aria-haspopup="menu" aria-expanded={open} {disabled} onclick={toggle}><Icon name="more"/></button>
 {#if open}<div bind:this={panel} class="menu-panel" role="menu" aria-label="Fixture actions" style:left={`${left}px`} style:top={`${top}px`}>
  {#if confirm}<p>Delete fixture?</p><button role="menuitem" class="danger" disabled={deleting} onclick={ondelete}>{deleting?'Deleting…':'Delete'}</button><button role="menuitem" disabled={deleting} onclick={close}>Cancel</button>
  {:else}{#if href}<a role="menuitem" {href} onclick={close}>Use fixture <Icon name="arrow" size={14}/></a>{/if}<button role="menuitem" class="danger" onclick={()=>confirm=true}>Delete</button>{/if}
 </div>{/if}
</div>
<style>
.fixture-menu{display:flex;justify-content:flex-end}.menu-trigger{display:inline-flex;align-items:center;justify-content:center;width:32px;height:32px;padding:0;border-radius:6px}.menu-panel{position:fixed;z-index:100;width:180px;padding:5px;border:1px solid var(--border);border-radius:8px;background:var(--surface,#fff);box-shadow:0 6px 20px #18253b18;text-align:left}.menu-panel a,.menu-panel button{display:flex;align-items:center;justify-content:space-between;width:100%;padding:9px 10px;border:0;border-radius:4px;background:transparent;color:var(--ink);font-size:12px;font-weight:450;text-decoration:none;text-align:left}.menu-panel a:hover,.menu-panel button:hover{background:var(--hover-bg)}.menu-panel .danger{color:var(--danger)}.menu-panel p{margin:6px 10px;font-size:12px;color:var(--muted)}
</style>
