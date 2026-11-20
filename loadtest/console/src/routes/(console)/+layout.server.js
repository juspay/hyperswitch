import {publicSettings} from '../../../server/state.mjs';
export function load({depends}){depends('console:settings');return {settings:publicSettings()};}
