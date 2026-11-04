import {writable} from 'svelte/store';
import {replaceState} from '$app/navigation';
export const navigationContext=writable({});
/** @type {Record<string,string>} */
let memory={};
export function rememberPage(url){
  if(typeof window==='undefined')return;
  const next={...memory};
  for(const key of ['profile','fixtures','run'])if(url.searchParams.has(key))next[key]=url.searchParams.get(key);
  if(['/performance','/results'].includes(url.pathname)&&!url.searchParams.has('run'))delete next.run;
  next[url.pathname]=url.pathname+url.search;memory=next;navigationContext.set(next);
}
export function sectionHref(section,url,context={}){
  const path=`/${section}`,next=new URL(context[path]||path,url.origin);
  for(const key of ['profile','fixtures','run']){
    const value=url.searchParams.get(key)||context[key];
    if(value&&(['profile','fixtures'].includes(key)||['performance','results'].includes(section)))next.searchParams.set(key,value);
  }
  if(!['performance','results'].includes(section)||!context.run&&!url.searchParams.has('run'))next.searchParams.delete('run');
  return next.pathname+next.search;
}
export function setQuery(values){
  const url=new URL(window.location.href);
  for(const [key,value] of Object.entries(values))if(value===null||value===undefined||value==='')url.searchParams.delete(key);else url.searchParams.set(key,String(value));
  replaceState(url,{});rememberPage(url);
}
