<script>
 import {tick,onMount} from 'svelte';
 import Icon from './Icon.svelte';
 export let id;
 export let text;
 export let label='More information';
 let open=false,button,bubble,left=12,top=12;
 async function show(){
  open=true;await tick();if(!open||!button||!bubble)return;
  const anchor=button.getBoundingClientRect(),box=bubble.getBoundingClientRect();
  left=Math.max(12,Math.min(anchor.left,window.innerWidth-box.width-12));
  top=anchor.bottom+8;
  if(top+box.height>window.innerHeight-12)top=Math.max(12,anchor.top-box.height-8);
 }
 onMount(()=>{document.addEventListener('scroll',hide,true);return()=>document.removeEventListener('scroll',hide,true);});
 function hide(){open=false;}
 function leave(){if(document.activeElement!==button)hide();}
</script>
<svelte:window onkeydown={event=>{if(open&&event.key==='Escape'){event.preventDefault();event.stopPropagation();hide();}}} onresize={hide} onscroll={hide}/>
<span class="info-tooltip">
 <button bind:this={button} type="button" class="info-trigger" aria-label={label} aria-describedby={open?id:undefined} onmouseenter={show} onmouseleave={leave} onfocus={show} onblur={hide} onclick={show}><Icon name="info" size={15}/></button>
 {#if open}<span bind:this={bubble} {id} role="tooltip" class="info-content" style:left={`${left}px`} style:top={`${top}px`}>{text}</span>{/if}
</span>
<style>
.info-tooltip{display:inline-flex;vertical-align:middle;margin-left:5px;font-size:12px;font-weight:400;line-height:1.5}
.info-trigger{display:inline-flex;align-items:center;justify-content:center;min-height:24px;min-width:24px;padding:3px;border:0;border-radius:4px;background:transparent;color:var(--muted);box-shadow:none;vertical-align:middle;cursor:help}
.info-trigger:hover{color:var(--accent);background:var(--accent-soft)}.info-trigger:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.info-content{position:fixed;z-index:1000;width:max-content;max-width:min(340px,calc(100vw - 24px));padding:10px 12px;border:1px solid var(--border);border-radius:6px;background:var(--surface,#fff);color:var(--ink);box-shadow:0 3px 12px #18253b14;font-size:12px;font-weight:400;line-height:1.6;white-space:normal;overflow-wrap:anywhere;text-align:left;pointer-events:none}
</style>
