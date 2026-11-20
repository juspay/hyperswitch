import { fail,redirect } from '@sveltejs/kit';
import { login,sessionCookie } from '../../../server/auth.mjs';
export const actions={
  default:async({request,cookies,getClientAddress,url})=>{
    const form=await request.formData();
    try {
      const token=await login(String(form.get('username') || ''),String(form.get('password') || ''),getClientAddress());
      if(!token)return fail(401,{error:'Invalid username or password'});
      cookies.set(sessionCookie,token,{path:'/',httpOnly:true,secure:url.protocol==='https:',sameSite:'strict',maxAge:8*3600});
    } catch(e) {return fail(e.status || 500,{error:e.status===429?e.message:'Login unavailable'});}
    redirect(303,'/');
  }
};
