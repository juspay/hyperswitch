<script>
export let name;
export let size=16;
const paths={
 settings:'M9.7 4.3 10.3 2h3.4l.6 2.3 2 .9 2.1-.7 1.7 2.9-1.5 1.6.2 2.2L21 13v3l-2.2.8-.9 2 0 2-3.2 1-1.5-1.7-2.2-.2-1.7 1.4-2.8-1.8.7-2.2-.8-2L4 14.5v-3l2.3-.8.8-2-.7-2.2 2-2.2 1.3 0Z',
 scenarios:'M4 4h6v6H4z M14 4h6v6h-6z M4 14h6v6H4z M14 14h6v6h-6z',
 fixtures:'M4 7l8-4 8 4-8 4-8-4Z M4 7v10l8 4 8-4V7 M12 11v10',
 performance:'M3 12h4l3-8 4 16 3-8h4',
 results:'M5 20V10 M12 20V4 M19 20v-7',
 logout:'M9 4H4v16h5 M14 7l5 5-5 5 M8 12h11',
 plus:'M12 5v14 M5 12h14',
 close:'M6 6l12 12 M18 6 6 18',
 arrow:'M5 12h14 M13 6l6 6-6 6',
 back:'M19 12H5 M11 6l-6 6 6 6',
 check:'M5 12l4 4L19 6',
 search:'M21 21l-5-5 M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
 more:'M5 12h.01 M12 12h.01 M19 12h.01',
 grip:'M8 5h.01 M16 5h.01 M8 12h.01 M16 12h.01 M8 19h.01 M16 19h.01',
 bolt:'m13 2-8 12h6l-1 8 9-13h-7l1-7Z',
 shield:'M12 3l8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Z M8 12l3 3 5-6',
 clock:'M12 8v5l3 2 M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0',
 chevron:'m8 5 7 7-7 7',
 info:'M12 11v6 M12 7h.01 M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0'
};
</script>
<svg class="ui-icon" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={paths[name]||paths.info}/>{#if name==='settings'}<circle cx="12" cy="12" r="3"/>{/if}</svg>
