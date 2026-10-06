import { createServer } from "node:http";
import { once } from "node:events";

export async function mockServer({baselinePolls=1,listPolls=1} = {}) {
 const state = {accounts:new Map(),connectors:new Map(),keys:0,patches:0,default:true,conflict:false,failConnector:false,requests:0,polls:new Map(),errorCustomers:false,malformedCustomers:false};
 const server = createServer(async(req,res) => {
  state.requests++;
  let text=''; for await (const chunk of req) text+=chunk;
  const body = text ? JSON.parse(text) : {};
  const url = new URL(req.url,'http://localhost');
  const send = (status,data) => {res.writeHead(status,{'content-type':'application/json','x-request-id':`r-${state.requests}`,'x-config-version':'v1'});res.end(JSON.stringify(data));};
  if(url.pathname==='/organization') return send(200,{organization_id:'org-1'});
  if(url.pathname==='/accounts' && req.method==='POST') {
   if(state.accounts.has(body.merchant_id)) return send(409,{error:{message:'exists'}});
   const account={merchant_id:body.merchant_id,organization_id:body.organization_id,default_profile:`pro-${body.merchant_id}`,publishable_key:'pk-test'};
   state.accounts.set(body.merchant_id,account); return send(200,account);
  }
  if(url.pathname.startsWith('/accounts/')) return send(200,state.accounts.get(url.pathname.split('/')[2]));
  if(url.pathname.startsWith('/api_keys/')) {state.keys++;return send(200,{api_key:`key-${state.keys}`});}
  if(url.pathname.endsWith('/connectors')) {
   const merchant=url.pathname.split('/')[2];
   if(req.method==='GET') return send(200,state.connectors.get(merchant)||[]);
   if(state.failConnector) return send(500,{error:{message:'connector unavailable'}});
   const connector={merchant_connector_id:`mca-${merchant}`,connector_label:'stripe_US_default'};
   state.connectors.set(merchant,[connector]);return send(200,connector);
  }
  if(url.pathname==='/default-config/unknown') return send(404,{error:{message:'unknown flag'}});
  if(url.pathname.startsWith('/default-config/')) {
   if(req.method==='PATCH') {state.patches++;state.default=body.value;}
   return send(200,{key:'flag',value:state.default,schema:{type:'boolean'}});
  }
  if(url.pathname==='/config/resolve') return send(200,{flag:state.conflict?false:state.default});
  // Local target for recorded k6 integration; IDs always unique.
  if(url.pathname==='/v2/customers') {
   if(state.errorCustomers) return send(422,{error:{message:'invalid customer'}});
   if(state.malformedCustomers) {res.writeHead(200,{'content-type':'application/json','x-request-id':'malformed-test'});res.end('not-json');return;}
   return send(200,{id:`cus-${state.requests}`});
  }
  if(url.pathname==='/v2/payment-method-sessions') return send(200,{id:`pms-${state.requests}`,client_secret:'secret-test'});
  if(url.pathname.startsWith('/v2/payment-method-sessions/')) return send(200,{associated_payment_methods:[{payment_method_token:'token-test'}]});
  if(url.pathname==='/payments' && req.method==='POST') return send(200,{payment_id:`pay-${state.requests}`,customer_id:body.customer_id,amount:body.amount,status:body.confirm?'succeeded':'requires_payment_method',client_secret:'secret-test'});
  if(url.pathname.endsWith('/confirm')) return send(200,{payment_id:url.pathname.split('/')[2],status:'succeeded'});
  if(url.pathname.endsWith('/client')) {
   const n=state.polls.get(url.pathname)||0;state.polls.set(url.pathname,n+1);
   return send(200,{customer_payment_methods:n<listPolls?[]:[{payment_token:'token-test'}]});
  }
  if(url.pathname==='/payments/session_tokens' || url.pathname.endsWith('/eligibility')) return send(200,{});
  if(url.pathname.startsWith('/payments/')) {
   const n=state.polls.get(url.pathname)||0;state.polls.set(url.pathname,n+1);
   return send(200,{payment_id:url.pathname.split('/')[2],...(n<baselinePolls?{}:{payment_method_id:'pm-test'}),status:'succeeded'});
  }
  return send(404,{error:{message:'unknown route'}});
 });
 server.listen(0,'127.0.0.1');await once(server,'listening');
 return {server,state,url:`http://127.0.0.1:${server.address().port}`};
}
