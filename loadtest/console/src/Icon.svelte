<script>
export let name;
export let size=16;
const paths={
 settings:'M19.391 8.939 L19.807 10.255 L21.759 9.819 L21.759 14.181 L19.807 13.745 L19.391 15.061 L19.391 15.061 L18.755 16.287 L20.443 17.358 L17.358 20.443 L16.287 18.755 L15.061 19.391 L15.061 19.391 L13.745 19.807 L14.181 21.759 L9.819 21.759 L10.255 19.807 L8.939 19.391 L8.939 19.391 L7.713 18.755 L6.642 20.443 L3.557 17.358 L5.245 16.287 L4.609 15.061 L4.609 15.061 L4.193 13.745 L2.241 14.181 L2.241 9.819 L4.193 10.255 L4.609 8.939 L4.609 8.939 L5.245 7.713 L3.557 6.642 L6.642 3.557 L7.713 5.245 L8.939 4.609 L8.939 4.609 L10.255 4.193 L9.819 2.241 L14.181 2.241 L13.745 4.193 L15.061 4.609 L15.061 4.609 L16.287 5.245 L17.358 3.557 L20.443 6.642 L18.755 7.713 L19.391 8.939Z',
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
