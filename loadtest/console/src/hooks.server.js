import { redirect, json } from '@sveltejs/kit';
import { authenticated, sessionCookie, sameOrigin } from '../server/auth.mjs';
import { initializeConfig } from '../server/state.mjs';
import { building } from '$app/environment';

export async function handle({ event, resolve }) {
  if(building)return resolve(event);
  initializeConfig();
  const path=event.url.pathname;
  if(path==='/health')return resolve(event);
  // Worker agents authenticate with their own bearer credentials and connect
  // outbound, so they do not have a browser session or Origin header.
  if(path.startsWith('/api/worker/v1/'))return resolve(event);
  if(!['GET','HEAD','OPTIONS'].includes(event.request.method) && !sameOrigin(event.request,event.url)) return json({error:'Invalid request origin'},{status:403});
  const isAuthenticated=authenticated(event.cookies.get(sessionCookie));
  if(path==='/login') {if(isAuthenticated && event.request.method==='GET')redirect(303,'/');return resolve(event);}
  if(!isAuthenticated) {
    if(path.startsWith('/api/')||path.startsWith('/grafana/'))return json({error:'Authentication required'},{status:401});
    redirect(303,'/login');
  }
  const response=await resolve(event);
  response.headers.set('cache-control','no-store');
  response.headers.set('x-content-type-options','nosniff');
  return response;
}
