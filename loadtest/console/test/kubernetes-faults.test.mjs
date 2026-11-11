import test from 'node:test';
import assert from 'node:assert/strict';
import {kubernetesAdapter} from '../server/kubernetes-faults.mjs';
function fixture(){
 const info={metadata:{uid:'deployment-uid',generation:1},spec:{replicas:2,selector:{matchLabels:{app:'vault'}}},status:{replicas:2,availableReplicas:2,readyReplicas:2,observedGeneration:1}};
 const pod=(name,uid,owner='ours')=>({metadata:{name,uid,ownerReferences:[{uid:owner,controller:true}]},spec:{nodeName:'node-1'}});
 const state={pods:[pod('vault-a','pod-a'),pod('vault-b','pod-b'),pod('unrelated','foreign','theirs')],running:['pod-a','pod-b'],denied:false,missing:false},calls=[];
 const command=async(binary,args)=>{
  assert.equal(binary,'kubectl');calls.push(args);
  if(args.includes('--raw')){if(state.denied)throw Error('Forbidden');return JSON.stringify({kind:'PodList',items:state.running.map(uid=>({metadata:{uid}}))});}
  const i=args.indexOf('get');if(i>=0){
   if(args[i+1]==='deployment')return JSON.stringify(info);
   if(args[i+1]==='replicasets')return JSON.stringify({items:[{metadata:{uid:'ours',ownerReferences:[{uid:'deployment-uid',controller:true}]}},{metadata:{uid:'theirs',ownerReferences:[{uid:'different',controller:true}]}}]});
   if(args[i+1]==='pods')return JSON.stringify({items:state.pods});
   if(args[i+1]==='pod')return state.missing?'':JSON.stringify(state.pods.find(p=>p.metadata.name===args[i+2]));
  }
  if(args.includes('scale')){info.spec.replicas=Number(args.find(a=>a.startsWith('--replicas=')).split('=')[1]);return '';}
  if(args.includes('delete')){state.pods=state.pods.filter(p=>p.metadata.name!==args[args.indexOf('pod')+1]);return '';}
  throw Error('Unexpected command');
 };
 const target={context:'explicit-context',namespace:'payments',deployment:'vault',outageMode:'hard'};
 return {adapter:kubernetesAdapter(target,command),state,info,calls,target,command};
}
test('hard outage terminates only captured deployment-owned pods after scale-zero',async()=>{
 const {adapter,calls,state}=fixture(),before=await adapter.probe();assert.deepEqual(before.pods.map(p=>p.uid),['pod-a','pod-b']);await adapter.apply('outage',before);
 const mutations=calls.filter(a=>a.includes('scale')||a.includes('delete'));assert.ok(mutations[0].includes('--replicas=0'));assert.equal(mutations.length,3);
 for(const args of mutations.slice(1)){for(const flag of ['--grace-period=0','--force','--wait=false'])assert.ok(args.includes(flag));assert.equal(args[1],'explicit-context');}
 assert.deepEqual(state.pods.map(p=>p.metadata.name),['unrelated']);
});
test('API deletion alone cannot start downtime while captured containers remain in runtime',async()=>{
 const {adapter,state,info}=fixture(),before=await adapter.probe();await adapter.apply('outage',before);Object.assign(info.status,{replicas:0,availableReplicas:0,readyReplicas:0});
 assert.equal(await adapter.isDown(before),false);state.running=[];assert.equal(await adapter.isDown(before),true);
 state.pods.push({metadata:{name:'replacement',uid:'new',ownerReferences:[{uid:'ours',controller:true}]},spec:{nodeName:'node-1'}});assert.equal(await adapter.isDown(before),false);
});
test('runtime permissions are preflighted, verification fails closed, recovery remains possible',async()=>{
 const {adapter,state,calls,info}=fixture();state.denied=true;await assert.rejects(adapter.probe(),/nodes\/proxy/);assert.equal(calls.some(a=>a.includes('scale')||a.includes('delete')),false);
 state.denied=false;const before=await adapter.probe();await adapter.apply('outage',before);Object.assign(info.status,{replicas:0,availableReplicas:0});state.denied=true;await assert.rejects(adapter.isDown(before),/Forbidden/);
 await adapter.restore(before);assert.equal(info.spec.replicas,2);
});
test('identity changes prevent mutation of replacement deployments',async()=>{
 const {adapter,info,calls}=fixture(),before=await adapter.probe();info.metadata.uid='replacement';await assert.rejects(adapter.apply('outage',before),/identity changed/);await assert.rejects(adapter.restore(before),/identity changed/);assert.equal(calls.some(a=>a.includes('scale')||a.includes('delete')),false);
});
test('scale-down may remove captured pods before the explicit deletion',async()=>{
 const {adapter,state,calls}=fixture(),before=await adapter.probe();state.missing=true;await adapter.apply('outage',before);assert.equal(calls.some(a=>a.includes('delete')),false);
});
test('graceful targets retain scale-only behavior',async()=>{
 const {target,command,calls,info}=fixture(),adapter=kubernetesAdapter({...target,outageMode:'graceful'},command),before=await adapter.probe();assert.equal(before.pods,undefined);await adapter.apply('outage',before);assert.equal(calls.some(a=>a.includes('--raw')||a.includes('delete')),false);await adapter.restore(before);assert.equal(info.spec.replicas,2);assert.equal(await adapter.verify(before),true);
});
