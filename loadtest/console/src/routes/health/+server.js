import { json } from '@sveltejs/kit';
import { controlDb } from '../../../server/state.mjs';
export function GET(){controlDb().prepare('SELECT 1').get();return json({status:'ok'});}
