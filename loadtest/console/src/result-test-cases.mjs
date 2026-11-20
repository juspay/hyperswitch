// Test-case presentation only. Recorded measurements and acceptance calculations
// remain in scenario-checks; repeated executions retain all of their evidence.
export const testCaseCatalog=[
  {id:'TC-01',title:'Calibration',section:'load'},
  {id:'TC-02',title:'Steady-load performance',section:'load'},
  {id:'TC-03',title:'Burst-load performance',section:'load'},
  {id:'TC-04',title:'Database failover',section:'faults'},
  {id:'TC-05',title:'Application reconnection',section:'faults'},
  {id:'TC-06',title:'Cache failover',section:'faults'},
  {id:'TC-07',title:'Cluster zone loss',section:'faults'},
  {id:'TC-08',title:'Cluster + database zone loss',section:'faults'},
  {id:'TC-09',title:'Multi-merchant load',section:'load'},
  {id:'TC-10',title:'Card vault restart',section:'faults'}
];
const byId=new Map(testCaseCatalog.map(tc=>[tc.id,tc]));
const intents={calibration:'TC-01',steady:'TC-02',burst:'TC-03'};
export function testCasePlan(config,groups){
  const plan=new Map();
  const add=(id,groupIds=[])=>{if(!byId.has(id))return;if(!plan.has(id))plan.set(id,{id,groupIds:[]});const entry=plan.get(id);entry.groupIds=[...new Set([...entry.groupIds,...groupIds])];};
  for(const group of groups){
    const stage=config.load?.stages?.[group.stageIndex]||{};
    if(['hold','ramp'].includes(group.kind)){
      const explicit=stage.testCase||intents[stage.intent];
      if(['TC-01','TC-02','TC-03'].includes(explicit)&&(explicit!=='TC-03'||group.kind==='hold'))add(explicit,[group.id]);
      else if(group.kind==='hold')add('TC-02',[group.id]);
    }
    if(group.kind==='fault'){
      const target=config.faultPlan?.find(f=>f.index===group.stageIndex)?.target||{};
      const action=config.faultPlan?.find(f=>f.index===group.stageIndex)?.action||stage.fault?.action;
      const type=target.serviceType||target.id||stage.fault?.targetId;
      const ids=(type==='postgres'&&action==='failover')||target.adapter==='gcp-alloydb'?['TC-04','TC-05']:(type==='redis'&&action==='failover')||target.adapter==='gcp-redis'?['TC-06']:type==='card-vault'?['TC-10']:type==='zone'?['TC-07']:type==='zone-and-database'?['TC-08']:[];
      for(const id of ids)add(id,[group.id,`recovery-${group.stageIndex}`,'integrity'].filter(key=>groups.some(g=>g.id===key)));
    }
  }
  const merchants=Number(config.fixtureSet?.merchantCount||0);
  if(config.mode==='tenancy'||merchants>1)add('TC-09',groups.filter(g=>['hold','ramp'].includes(g.kind)).map(g=>g.id));
  // Explicit selections without execution evidence remain Not run. In
  // particular, a ramp is never inferred to be a sustained burst or a zone loss.
  for(const id of config.load?.testCases||[])add(id);
  return [...plan.values()].map(entry=>({...entry,...(entry.id==='TC-09'?{merchantCount:merchants||null}:{})}));
}
const number=value=>Number(value).toLocaleString(undefined,{maximumFractionDigits:2});
const value=check=>check?.actual==null?'—':`${check.lowerBound?'≥ ':''}${number(check.actual)}${check.unit==='%'?'%':` ${check.unit||''}`}`;
const threshold=check=>check?.limit==null?'—':`${check.comparison} ${number(check.limit)}${check.unit==='%'?'%':` ${check.unit||''}`}`;
const missing=(id,label,limit=null,unit='')=>({id,label,actual:null,limit,unit,comparison:'≤',outcome:'not_measured'});
const status=checks=>checks.some(c=>c.outcome==='fail')?'fail':checks.some(c=>c.outcome==='in_progress')?'in_progress':checks.every(c=>c.outcome==='not_run')?'not_run':checks.some(c=>['not_measured','not_run'].includes(c.outcome))?'not_measured':checks.some(c=>c.outcome==='pass')?'pass':'recorded';
function extreme(checks,id,minimum=false){return checks.filter(c=>c.id===id&&c.actual!=null).reduce((a,b)=>!a||(minimum?b.actual<a.actual:b.actual>a.actual)?b:a,null);}
export function resultTestCases(summary){
  const groups=summary.groups||[];
  // Older cached payloads can still render without changing any measurements.
  const plan=summary.testCasePlan||testCasePlan({load:{},faultPlan:groups.filter(g=>g.kind==='fault').map(g=>({index:g.stageIndex,target:{id:/vault/i.test(g.target||g.title)?'card-vault':/postgres|database|\bdb\b/i.test(g.target||g.title)?'postgres':'unknown'}}))},groups);
  const rows=plan.map(entry=>{
    const catalog=byId.get(entry.id);if(!catalog)return null;
    const parts=entry.groupIds.flatMap(id=>groups.find(g=>g.id===id)||[]);
    const checks=entry.id==='TC-02'&&summary.steady?summary.steady.checks.map(c=>({...c,source:'steady',sourceLabel:'Steady load'})):parts.flatMap(g=>g.checks.map(c=>({...c,source:g.id,sourceLabel:g.title})));
    let expected='Saved limits',observed='Pending verification';
    const loadChecks=entry.id==='TC-02'&&summary.steady?checks:parts.filter(g=>['hold','ramp'].includes(g.kind)).flatMap(g=>g.checks);
    const faultChecks=parts.filter(g=>g.kind==='fault').flatMap(g=>g.checks);
    if(['TC-02','TC-03'].includes(entry.id)){
      const throughput=extreme(loadChecks,'throughput',true),errors=extreme(loadChecks,'errors');
      expected=throughput?.limit!=null?`Throughput ${threshold(throughput)}`:'Record throughput';
      observed=[throughput?`${value(throughput)} window throughput`:null,errors?`${value(errors)} HTTP errors`:null].filter(Boolean).join(' · ')||'Pending verification';
    }else if(entry.id==='TC-01'){
      checks.push(missing('calls-per-payment','API calls per payment'));
      expected='Record API calls per payment';observed='Call-counting evidence not available';
    }else if(entry.id==='TC-09'){
      checks.push(missing('merchant-coverage','Verified merchant traffic coverage'));
      expected='Traffic across all configured merchants';observed=entry.merchantCount?`${number(entry.merchantCount)} merchants configured`:'Merchant coverage pending verification';
    }else{
      const recovery=extreme(faultChecks,'recovery'),failureCounts=faultChecks.filter(c=>c.id==='failed');
      expected=recovery?.limit!=null?`Recovery ${threshold(recovery)}`:'Record recovery time';
      observed=recovery?`${value(recovery)} recovery`:'Pending verification';
      if(failureCounts.length){
        for(let i=checks.length-1;i>=0;i--)if(checks[i].id==='failed')checks.splice(i,1);
        checks.push({id:'total-failed',label:'Total failed requests',actual:failureCounts.every(c=>c.actual!=null)?failureCounts.reduce((sum,c)=>sum+c.actual,0):null,limit:null,unit:'requests',comparison:'≤',outcome:'recorded'});
      }
      if(entry.id==='TC-05'){
        // A successful payment confirmation after DB recovery proves that
        // application payment traffic resumed; it makes no claim about pods.
        const recoveries=parts.filter(g=>g.kind==='recovery');
        const reconnect=recoveries.map(g=>{
          const actual=g.checks.find(c=>c.id==='completed')?.actual??null;
          return {id:'application-reconnection',label:'Successful payment confirmations after recovery',actual,limit:1,unit:'confirmations',comparison:'≥',source:g.id,sourceLabel:g.title,
            outcome:g.state==='not_run'?'not_run':actual==null?'not_measured':actual>0?'pass':g.state==='complete'?'fail':'in_progress'};
        });
        checks.splice(0,checks.length,...(reconnect.length?reconnect:[missing('application-reconnection','Successful payment confirmations after recovery',1,'confirmations')]));
        expected='Payment confirmations resume after failover';
        observed=reconnect.length&&reconnect.every(c=>c.actual!=null)?`${number(reconnect.reduce((sum,c)=>sum+c.actual,0))} successful confirmations after recovery`:'Pending verification';
      }
    }
    const failedLatency=checks.find(c=>/^p(50|95|99)$/.test(c.id)&&c.outcome==='fail');
    const failureReason=failedLatency?`${failedLatency.source?.startsWith('recovery-')?'Recovery':entry.id==='TC-02'?'Steady':''} ${failedLatency.id} exceeds ${number(failedLatency.limit)} ms`.trim():null;
    const executed=parts.some(g=>['hold','ramp','fault'].includes(g.kind)&&g.state!=='not_run');
    return {...catalog,expected,failureReason,observed:executed?observed:'Not run',checks,outcome:executed?status(checks):'not_run',occurrences:parts.filter(g=>g.kind==='fault'||['hold','ramp'].includes(g.kind)).length};
  }).filter(Boolean);
  // Preserve ramp/unknown-adapter measurements without incorrectly assigning
  // them an original TC whose prerequisite is absent.
  const assigned=new Set(plan.flatMap(p=>p.groupIds));
  for(const [kind,title,section] of [['ramp','Ramp observations','load'],['fault','Fault observations','faults']]){
    const parts=groups.filter(g=>g.kind===kind&&!assigned.has(g.id));
    if(parts.length){const checks=parts.flatMap(g=>g.checks.map(c=>({...c,source:g.id,sourceLabel:g.title})));rows.push({id:`observations-${kind}`,title,section,expected:'Saved limits',observed:`${parts.length} ${kind==='ramp'?'ramp':'fault'} period${parts.length===1?'':'s'}`,checks,outcome:status(checks)});}
  }
  rows.sort((a,b)=>(testCaseCatalog.findIndex(tc=>tc.id===a.id)+1||100)-(testCaseCatalog.findIndex(tc=>tc.id===b.id)+1||100));
  if(summary.completion){
    const {expected,successful,check}=summary.completion;
    rows.unshift({id:'completion',title:summary.completion.scope?'Payment confirmations':'Flow completion',section:'load',expected:check.limit==null?'Recorded target':`${number(check.limit)}% ${summary.completion.scope?'outside outage':'of target flows'}`,
      observed:check.actual==null?'—':`${number(successful)} / ${number(expected)} ${summary.completion.scope?'confirmations':'flows'} · ${value(check)}${summary.completion.outageFailures?` · ${number(summary.completion.outageFailures)} failed during outage`:''}`,
      checks:[check],outcome:check.outcome});
  }
  return [{id:'load',title:'Load & performance'},{id:'faults',title:'Faults & recovery'}].map(section=>({...section,cases:rows.filter(row=>row.section===section.id)})).filter(section=>section.cases.length);
}
export {value as formatTestCaseValue,threshold as formatTestCaseLimit};
