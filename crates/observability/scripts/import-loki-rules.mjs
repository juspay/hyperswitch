#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

// Imports source definitions only. No query execution, notification or remote-rule mutation.
export function convertRule(rule, destination, interval_seconds=rule.intervalSeconds) {
  if(!Number.isInteger(interval_seconds)||interval_seconds<60||interval_seconds>86400||interval_seconds%60!==0)throw Error('Verified source cadence must be whole minutes from 60 to 86400 seconds; no cadence normalization');
  const data = rule.data ?? [];
  const sources = data.filter(d => d.datasourceUid === 'loki');
  if (sources.length !== 1) throw Error('Exactly one Loki source is required');
  const source = sources[0], model = source.model;
  const byRef = new Map(data.map(d => [d.refId,d]));
  let current = byRef.get(rule.condition), reducer='last', threshold=0, comparison='GreaterThanThreshold';
  const blockers=[];
  const operators={gt:'GreaterThanThreshold',gte:'GreaterThanOrEqualToThreshold',lt:'LessThanThreshold',lte:'LessThanOrEqualToThreshold'};
  const reducers={last:'last',count:'count',sum:'sum',min:'minimum',max:'maximum',mean:'mean'};
  if (!current) throw Error('Condition reference missing');
  if (current.refId === source.refId) {
    blockers.push('Direct source condition needs explicit Grafana semantics review; nonzero is preview only');
  } else {
    const node=current.model, conditions=node.conditions;
    if(node.type!=='threshold'||conditions?.length!==1)throw Error('Unsupported threshold graph');
    const evaluator=conditions[0].evaluator;
    if(!operators[evaluator?.type]||evaluator.params?.length!==1||!Number.isFinite(evaluator.params[0]))throw Error('Unsupported threshold parameters');
    comparison=operators[evaluator.type];threshold=evaluator.params[0];
    current=byRef.get(node.expression);
    if(!current)throw Error('Missing threshold input');
    if(current.refId!==source.refId){
      if(current.model.type!=='reduce'||!reducers[current.model.reducer])throw Error('Unsupported reduction');
      if(current.model.settings?.mode && current.model.settings.mode!=='strict')throw Error('Non-strict reducer not supported');
      reducer=reducers[current.model.reducer];
      current=byRef.get(current.model.expression);
    }
    if(current?.refId!==source.refId)throw Error('Expression graph not fully represented');
  }
  const pending = rule.for ?? '0s';
  if(!/^\d+[smh]$/.test(pending))throw Error('Unsupported pending duration');
  const units={s:1,m:60,h:3600},pending_seconds=Number(pending.slice(0,-1))*units[pending.at(-1)];
  if(pending_seconds)blockers.push('Persistent pending timers not implemented');
  if((rule.keep_firing_for??rule.keepFiringFor??'0s')!=='0s')blockers.push('Keep-firing policy not implemented');
  if(rule.noDataState!=='OK')blockers.push('Source no-data policy needs lifecycle support');
  if(rule.execErrState!=='OK')blockers.push('Source query-error policy needs lifecycle support');
  const mode=model.queryType??(model.instant?'instant':'range');
  if(!['instant','range'].includes(mode))throw Error('Unsupported query mode');
  if(mode==='range'&&model.instant===true)blockers.push('Conflicting source instant/range flags need verification');
  if(source.relativeTimeRange?.to!==0)throw Error('Nonzero end offset not supported');
  const range_seconds=source.relativeTimeRange.from;
  if(!Number.isInteger(range_seconds)||range_seconds<1||range_seconds>86400)throw Error('Unsupported lookback');
  const step_seconds=(model.intervalMs??1000)/1000;
  if(!Number.isInteger(step_seconds)||step_seconds<1)throw Error('Unsupported query step');
  // Even representable conditions need scope, routing, timing and field review before sends.
  blockers.push('Manual rule parity/routing review required before opting into delivery');
  return {
    uid:rule.uid,title:rule.title,blockers,
    body:{name:rule.title,status:'active',alerts_configuration:{
      version:'v1',interval_seconds,
      query:{datasource:'loki',version:'v1',expression:model.expr,mode,range_seconds,step_seconds,reducer,pending_seconds,delivery_enabled:false},
      thresholds:{version:'v1',severities:{critical:{threshold,comparison_operator:comparison,description:rule.annotations?.description||rule.annotations?.summary||rule.title,destination}}}
    }},
    source_policy:{for:rule.for,keep_firing_for:rule.keep_firing_for??rule.keepFiringFor??'0s',noDataState:rule.noDataState,execErrState:rule.execErrState,labels:rule.labels,condition:rule.condition,interval_seconds}
  };
}

export async function importRules(argv=process.argv.slice(2),env=process.env){
  const arg=name=>{const i=argv.indexOf(name);return i<0?undefined:argv[i+1]};
  const file=arg('--source'),groupFile=arg('--groups'),destination=arg('--destination'),output=arg('--report');
  if(!file||!groupFile||!destination||!output)throw Error('Require --source JSON --groups JSON --destination CONFIGURED_ID --report JSON');
  const origin=new URL(env.LOCAL_OBSERVABILITY_URL??'http://127.0.0.1:8086');
  if(origin.protocol!=='http:'||!['localhost','127.0.0.1','[::1]'].includes(origin.hostname)||origin.username||origin.password||origin.pathname!=='/')throw Error('Only a loopback application may be seeded');
  if(!env.LOCAL_OBSERVABILITY_API_KEY)throw Error('LOCAL_OBSERVABILITY_API_KEY required');
  const source=JSON.parse(fs.readFileSync(file,'utf8'));
  const groups=JSON.parse(fs.readFileSync(groupFile,'utf8'));
  const cadences=new Map();
  for(const list of Object.values(groups))for(const group of list)for(const rule of group.rules??[]) {
    const uid=rule.grafana_alert?.uid??rule.uid;
    const match=group.interval?.match(/^(\d+)([smh])$/);
    const seconds=match?Number(match[1])*({s:1,m:60,h:3600}[match[2]]):null;
    if(cadences.has(uid))throw Error('Duplicate source group UID');
    cadences.set(uid,seconds);
  }
  const rules=(Array.isArray(source)?source:source.rules).filter(r=>r.isPaused===false);
  if(new Set(rules.map(r=>r.uid)).size!==rules.length)throw Error('Duplicate source UID');
  const report={at:new Date().toISOString(),source_file:path.resolve(file),group_file:path.resolve(groupFile),rules:[]};
  fs.mkdirSync(path.dirname(path.resolve(output)),{recursive:true});
  for(const rule of rules){
    try{
      const converted=convertRule(rule,destination,cadences.get(rule.uid)??null);
      const response=await fetch(new URL('/alerts/rules/'+encodeURIComponent(rule.uid),origin),{method:'PUT',headers:{'X-Internal-Api-Key':env.LOCAL_OBSERVABILITY_API_KEY,'Content-Type':'application/json'},body:JSON.stringify(converted.body),redirect:'error',signal:AbortSignal.timeout(15000)});
      if(!response.ok)throw Error('LOCAL_HTTP_'+response.status);
      report.rules.push({uid:rule.uid,title:rule.title,status:'stored_preview_only',blockers:converted.blockers,source_policy:converted.source_policy});
    }catch(e){report.rules.push({uid:rule.uid,title:rule.title,status:'not_imported',reason:e.message});}
    fs.writeFileSync(output,JSON.stringify(report,null,2),{mode:0o600});
  }
  console.log(JSON.stringify({source_enabled_rules:rules.length,stored:report.rules.filter(r=>r.status==='stored_preview_only').length,not_imported:report.rules.filter(r=>r.status==='not_imported').length,report:output}));
  return report;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(path.resolve(process.argv[1])).href){importRules().catch(()=>{console.error('Local import failed; no alert delivery was enabled.');process.exitCode=1;});}
