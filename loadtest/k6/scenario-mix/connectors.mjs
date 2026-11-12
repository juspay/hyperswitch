export function connectorSchedule(plan){
 if(!plan)return null;
 if(!Array.isArray(plan.mix)||!plan.mix.length)throw new Error('Connector mix is empty');
 const rows=plan.mix;let total=0;const slots=[];
 for(const row of rows){if(!Number.isInteger(row.weight)||row.weight<1||row.weight>100)throw new Error('Invalid connector weight');total+=row.weight;}
 if(total!==100)throw new Error('Connector weights must total 100%');
 const current=rows.map(()=>0);for(let i=0;i<100;i++){for(let j=0;j<rows.length;j++)current[j]+=rows[j].weight;let winner=0;for(let j=1;j<rows.length;j++)if(current[j]>current[winner])winner=j;slots.push(rows[winner].connector);current[winner]-=100;}return slots;
}
export function selectedConnector(slots,iteration,merchant){
 if(!slots)return null;
 const cycle=Math.floor(iteration/100),name=slots[(iteration+cycle)%100];
 const account=merchant.connector_accounts?.[name]||(name==='stripe_test'&&slots.every(c=>c===name)?merchant.merchant_connector_id:null);
 if(!account)throw new Error(`Fixture has no ${name} connector account`);
 return {connector:name,merchant_connector_id:account};
}
export const connectorRouting=choice=>choice?{routing:{type:'single',data:choice}}:{};

// Synthetic browser context for card sandbox authorizations (including non-3DS
// connectors such as Adyen). Explicit payment configuration can override it.
export const connectorBrowserInfo=(overrides={})=>({accept_header:'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',language:'en-US',screen_height:1080,screen_width:1920,color_depth:24,user_agent:'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/131.0.0.0 Safari/537.36',time_zone:0,java_enabled:false,java_script_enabled:true,ip_address:'127.0.0.1',...overrides});

// Provider sandbox cards are test fixtures, separate from connector identity
// and credentials. Adyen's successful non-3DS card is also used by
// cypress-tests/cypress/e2e/configs/Payment/Adyen.js; Stripe's 4242 card is
// deliberately a refusal card there. Explicit overrides allow negative tests.
const sandboxCards={adyen:{card_number:'4111111111111111',card_exp_month:'03',card_exp_year:'30',card_holder_name:'Load Test',card_cvc:'737'}};
export const connectorTestCard=(connector,fallback,overrides={})=>overrides[connector]||sandboxCards[connector]||fallback;
