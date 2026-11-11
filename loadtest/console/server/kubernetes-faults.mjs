// A deleted API object is not proof that its containers have stopped. For hard
// outages, verify captured pod UIDs against each node's kubelet runtime list.
export function kubernetesAdapter(target,command){
 const t=target,context=t.context?['--context',t.context]:[],flags=[...context,'--namespace',t.namespace];
 const json=async args=>JSON.parse(await command('kubectl',args));
 const inspect=()=>json([...flags,'get','deployment',t.deployment,'-o','json']);
 const replicas=before=>t.recoveryReplicas??before.replicas;
 const runtime=async node=>{
  if(!/^[a-z0-9][a-z0-9.-]*$/.test(node))throw Error('Invalid Kubernetes node name');
  const data=await json([...context,'get','--raw',`/api/v1/nodes/${node}/proxy/runningpods/`]);
  if(data.kind!=='PodList'||!Array.isArray(data.items))throw Error('Kubelet did not return a runtime pod list');
  return data.items;
 };
 const selector=info=>{
  const s=info.spec.selector;
  const fields=Object.entries(s?.matchLabels||{}).map(([key,value])=>`${key}=${value}`);
  for(const expression of s?.matchExpressions||[]){
   const {key,operator,values=[]}=expression;
   if(operator==='In'||operator==='NotIn')fields.push(`${key} ${operator==='In'?'in':'notin'} (${values.join(',')})`);
   else if(operator==='Exists')fields.push(key);
   else if(operator==='DoesNotExist')fields.push(`!${key}`);
   else throw Error('Unsupported deployment label selector');
  }
  if(!fields.length)throw Error('Deployment has no pod selector');
  return fields.join(',');
 };
 const ownedPods=async info=>{
  const sets=await json([...flags,'get','replicasets','--selector',selector(info),'-o','json']);
  const owners=new Set(sets.items.filter(r=>r.metadata.ownerReferences?.some(o=>o.controller&&o.uid===info.metadata.uid)).map(r=>r.metadata.uid));
  const pods=await json([...flags,'get','pods','--selector',selector(info),'-o','json']);
  return pods.items.filter(p=>p.metadata.ownerReferences?.some(o=>o.controller&&owners.has(o.uid)));
 };
 const unchanged=async before=>{const info=await inspect();if(info.metadata.uid!==before.uid)throw Error('Deployment identity changed');return info;};
 return {
  probe:async()=>{
   const info=await inspect(),count=info.spec.replicas;
   if(!(count>0)||info.status.availableReplicas!==count||info.status.observedGeneration<info.metadata.generation)throw Error('Deployment is not ready');
   const before={replicas:count,uid:info.metadata.uid};
   if(t.outageMode==='hard'){
    const pods=await ownedPods(info);
    if(!pods.length)throw Error('No deployment-owned vault pods found');
    before.pods=pods.map(p=>({name:p.metadata.name,uid:p.metadata.uid,node:p.spec.nodeName}));
    // Check runtime access before making any infrastructure change.
    for(const node of new Set(before.pods.map(p=>p.node))){
     let running;try{running=await runtime(node);}catch{throw Error('Hard outage requires read access to kubelet runtime pods (nodes/proxy). Check Kubernetes permissions.');}
     if(before.pods.filter(p=>p.node===node).some(p=>!running.some(r=>r.metadata.uid===p.uid)))throw Error('Vault pod runtime identity could not be verified');
    }
    before.outageMode='hard';
   }
   return before;
  },
  apply:async(action,before)=>{
   await unchanged(before);
   await command('kubectl',[...flags,'scale','deployment',t.deployment,'--replicas=0']);
   if(before.outageMode==='hard')for(const pod of before.pods){
    if(!/^[a-z0-9][a-z0-9.-]*$/.test(pod.name))throw Error('Invalid captured pod name');
    const text=await command('kubectl',[...flags,'get','pod',pod.name,'--ignore-not-found=true','-o','json']);
    const current=text.trim()?JSON.parse(text):null;
    if(current?.metadata&&current.metadata.uid!==pod.uid)throw Error('Vault pod identity changed');
    if(current?.metadata)await command('kubectl',[...flags,'delete','pod',pod.name,'--grace-period=0','--force','--wait=false','--ignore-not-found=true']);
   }
  },
  isDown:async before=>{
   const info=await unchanged(before);
   if(info.spec.replicas!==0||(info.status.replicas||0)!==0||(info.status.availableReplicas||0)!==0||info.status.observedGeneration<info.metadata.generation)return false;
   if(before.outageMode==='hard'){
    if((await ownedPods(info)).length)return false;
    for(const node of new Set(before.pods.map(p=>p.node))){
     const running=await runtime(node);
     if(before.pods.some(p=>p.node===node&&running.some(r=>r.metadata.uid===p.uid)))return false;
    }
   }
   return true;
  },
  restore:async before=>{await unchanged(before);await command('kubectl',[...flags,'scale','deployment',t.deployment,`--replicas=${replicas(before)}`]);},
  verify:async before=>{const info=await inspect(),count=replicas(before);return info.metadata.uid===before.uid&&info.spec.replicas===count&&(info.status.availableReplicas||0)>=count&&(info.status.readyReplicas??info.status.availableReplicas??0)>=count&&info.status.observedGeneration>=info.metadata.generation;}
 };
}
