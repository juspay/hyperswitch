import {toStore} from 'svelte/store';
import {page as kitPage,navigating as kitNavigating} from '$app/state';

// Read framework context only when a component subscribes, never at module load.
// Each subscription gets its own snapshot and reactive lifetime, including SSR.
const contextStore=read=>({subscribe(run,invalidate){return toStore(read).subscribe(run,invalidate);}});
export const page=contextStore(()=>({...kitPage}));
export const navigating=contextStore(()=>kitNavigating.to?{...kitNavigating}:null);
