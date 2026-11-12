import { redirect } from '@sveltejs/kit';
import { logout,sessionCookie } from '../../../server/auth.mjs';
export function POST({cookies}){logout(cookies.get(sessionCookie));cookies.delete(sessionCookie,{path:'/'});redirect(303,'/login');}
